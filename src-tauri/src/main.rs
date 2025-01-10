#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod api;
mod video;
mod logger;
mod metadata;

use walkdir::WalkDir;
use tauri::{Manager, State};
use db::{DbState, VideoInfo};
use std::{
    env, 
    fs::{self, File}, 
    io::{self, BufRead},
    process::Command, 
    sync::{Arc, Mutex}
};
use serde::{Deserialize, Serialize};

// 导出日志宏
pub use crate::logger::{log_error, log_info, log_debug};

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "mov"];

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Settings {
    player_path: Option<String>,
    player_type: Option<String>,
    auto_subtitle: Option<bool>,
    /**
     * eng - 英语 (English)
     * fre - 法语 (French)
     * spa - 西班牙语 (Spanish)
     * ger - 德语 (German)
     * ita - 意大利语 (Italian)
     * jpn - 日语 (Japanese)
     * kor - 韩语 (Korean)
     * chi - 中文 (Chinese)
     * rus - 俄语 (Russian)
     * por - 葡萄牙语 (Portuguese)
     */
    subtitle_language: Option<String>, // 添加字幕语言字段
    tmdb_api_key: Option<String>,
    auto_tmdb: Option<bool>,
}

struct AppState {
    settings: Arc<Mutex<Settings>>,
}

#[tauri::command]
async fn scan_folder(path: String, db: State<'_, DbState>, settings: Settings) -> Result<Vec<VideoInfo>, String> {
    let db = db.0.clone();
    let new_videos = Arc::new(Mutex::new(Vec::new()));

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            if let Some(ext) = e.path().extension() {
                // 忽略非视频文件和示例视频
                VIDEO_EXTENSIONS.contains(&ext.to_string_lossy().to_lowercase().as_str()) &&
                !e.file_name().to_string_lossy().to_ascii_lowercase().contains("sample")
            } else {
                false
            }
        })
    {
        let path = entry.path().to_owned();
        let id = format!("{:x}", md5::compute(path.to_string_lossy().as_bytes()));
        
        // 检查视频是否已存在
        let db_clone = db.clone();
        let id_clone = id.clone();
        let exists = match tokio::task::spawn_blocking(move || {
            let conn = db_clone.lock().unwrap();
            db::video_exists(&conn, &id_clone)
        }).await {
            Ok(result) => result,
            Err(e) => {
                log_error!("Failed to check video existence: {}", e);
                continue;
            }
        };

        if !exists {
            let file_name = path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            
            // 解析剧集信息
            let series_info = video::parse_series_info(&file_name);
            let search_name = if series_info.is_series {
                &series_info.series_title
            } else {
                &file_name
            };
            
            // 获取视频时长
            let formatted_duration = video::get_duration(&path.to_string_lossy());
            
            let mut video_info_str = String::new();
            if settings.auto_tmdb.unwrap_or(false) {
                // 获取 TMDb 信息
                if let Some(ref api_key) = settings.tmdb_api_key {
                    video_info_str = video::fetch_video_info_from_tmdb(&search_name, api_key).await?;
                }
            }

            log_debug!("video_info_str: {}", video_info_str);
            if video_info_str.is_empty() {
                video_info_str = serde_json::json!({
                    "title": search_name,
                    "original_title": search_name,
                    "overview": "未找到匹配的电影信息",
                    "release_date": "",
                    "poster_path": "/assets/no-poster.png",
                    "vote_average": 0.0,
                    "genres": "未分类",
                }).to_string();
            }
            let video_info = match serde_json::from_str::<serde_json::Value>(&video_info_str) {
                Ok(info) => info,
                Err(e) => {
                    log_error!("Failed to parse video info: {}", e);
                    continue;
                }
            };
            let video = VideoInfo {
                id: id,
                title: video_info.get("original_title").and_then(|v| v.as_str()).unwrap_or(&file_name).to_string(),
                title_cn: video_info.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                thumbnail: video_info.get("poster_path").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                duration: formatted_duration.unwrap_or_else(|_| "Unknown".to_string()),
                path: path.to_string_lossy().to_string(),
                category: if series_info.is_series { "剧集" } else { "电影" }.to_string(),
                description: video_info.get("overview").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                create_time: chrono::Utc::now().timestamp(),
                last_play_time: 0,
                play_count: 0,
                favorite: false,
                tags: video_info.get("genres").and_then(|v| v.as_str()).unwrap_or("未分类").to_string(),
                is_series: series_info.is_series,
                series_title: series_info.series_title,
                season: series_info.season,
                episode: series_info.episode,
            };

            let binding = db.clone();
            let new_videos_clone = new_videos.clone();
            match tokio::task::spawn_blocking(move || {
                let conn = binding.lock().unwrap();
                let _ = db::insert_video(&conn, &video);
                new_videos_clone.lock().unwrap().push(video);
            }).await {
                Ok(result) => result,
                Err(e) => {
                    log_error!("Failed to check video existence: {}", e);
                    continue;
                }
            };
        }
    }

    // 最后获取所有视频
    tokio::task::spawn_blocking(move || {
        let conn = db.lock().unwrap();
        db::get_all_videos(&conn).map_err(|e| e.to_string())
    }).await.unwrap()
}

#[tauri::command]
async fn select_and_scan_folder(app_state: State<'_, AppState>, db: State<'_, DbState>) -> Result<Vec<VideoInfo>, String> {
    if let Some(path) = rfd::FileDialog::new().pick_folder() {
        let settings = {
            let settings_guard = app_state.settings.lock().unwrap();
            settings_guard.clone()
        };
        scan_folder(path.to_string_lossy().to_string(), db, settings).await
    } else {
        Ok(vec![]) // 用户取消选择
    }
}

#[tauri::command]
async fn get_cached_videos(db: State<'_, DbState>) -> Result<Vec<VideoInfo>, String> {
    let conn = match db.0.try_lock() {
        Ok(lock) => lock,
        Err(_) => return Err("Failed to acquire database lock".to_string()),
    };
    db::get_all_videos(&conn).map_err(|e| e.to_string())
}



#[tauri::command]
async fn play_video(mut video: VideoInfo, app_handle: tauri::AppHandle) -> Result<(), String> {
    let app_state = app_handle.state::<AppState>();
    let settings = app_state.settings.lock().unwrap();
    let path = video.path.clone();
    let subtitle_path = video::find_subtitles(&video).unwrap_or_default();

    video.play_count += 1;
    video.last_play_time = chrono::Utc::now().timestamp();
    update_video(app_handle.state::<DbState>(), video).map_err(|e| e.to_string())?;

    // 检查是否自动加载字幕
    let auto_subtitle = settings.auto_subtitle.clone().unwrap_or(false);
    let subtitle_language = settings.subtitle_language.clone().unwrap_or_else(|| "eng".to_string());

    match &settings.player_path {
        Some(player_path) if !player_path.is_empty() => {
            match settings.player_type.as_deref() {
                Some("vlc") => {
                    let mut command = Command::new(player_path);
                    command.arg(&path); // 指定视频文件
                    
                    if auto_subtitle {
                        command.arg("--sub-file").arg(&subtitle_path); // 指定字幕文件
                    }
                    command.arg("--sub-language").arg(&subtitle_language); // 指定字幕语言
                    command.arg("--fullscreen"); // 全屏播放（可选）
                    command.spawn().map_err(|e| e.to_string())?;
                }
                _ => {
                    eprintln!("Unsupported player type: {:?}", settings.player_type);
                }
                
            }
        }
        _ => {
            // 如果没有设置播放器路径，使用系统默认播放器
            #[cfg(target_os = "windows")]
            let status = Command::new("cmd")
                .arg("/C")
                .arg("start")
                .arg(&path)
                .status()
                .expect("Failed to open video");
        
            #[cfg(target_os = "macos")]
            let status = Command::new("open")
                .arg(&path)
                .status()
                .expect("Failed to open video");
        
            #[cfg(target_os = "linux")]
            let status = Command::new("xdg-open")
                .arg(&path)
                .status()
                .expect("Failed to open video");

            if !status.success() {
                eprintln!("Failed to open video");
            }
        }
    }
    Ok(())
}

#[tauri::command]
async fn save_settings(settings: Settings, app_handle: tauri::AppHandle) -> Result<(), String> {
    let config_dir = app_handle.path().app_config_dir().unwrap();

    println!("settings: {}", serde_json::to_string_pretty(&settings).unwrap());
    fs::create_dir_all(&config_dir)
        .map_err(|e| e.to_string())?;
    
    let settings_path = config_dir.join("settings.json");
    let settings_str = serde_json::to_string_pretty(&settings)
        .map_err(|e| e.to_string())?;
    
    fs::write(settings_path, settings_str)
        .map_err(|e| e.to_string())?;

    // 更新全局状态中的 settings
    let app_state = app_handle.state::<AppState>();
    let mut global_settings = app_state.settings.lock().unwrap();
    *global_settings = settings;
    
    Ok(())
}

#[tauri::command]
async fn load_settings(app_handle: tauri::AppHandle) -> Result<Settings, String> {
    let config_dir = app_handle.path().app_config_dir().unwrap();
    let settings_path = config_dir.join("settings.json");
    
    if settings_path.exists() {
        let settings_str = fs::read_to_string(settings_path)
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&settings_str)
            .map_err(|e| e.to_string())
    } else {
        Ok(Settings {
            player_path: None,
            player_type: Some("system".to_string()),
            auto_subtitle: Some(true),
            subtitle_language: Some("eng".to_string()),
            tmdb_api_key: None,
            auto_tmdb: Some(false),
        })
    }
}

#[tauri::command]
fn remove_video(db: State<'_, DbState>, id: String) -> Result<(), String> {
    let conn = match db.0.try_lock() {
        Ok(lock) => lock,
        Err(_) => return Err("Failed to acquire database lock".to_string()),
    };
    db::delete_video(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_video(db: State<'_, DbState>, video: VideoInfo) -> Result<(), String> {
    let conn = match db.0.try_lock() {
        Ok(lock) => lock,
        Err(_) => return Err("Failed to acquire database lock".to_string()),
    };
    db::update_video(&conn, &video).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_video_duration(path: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        video::get_duration(&path).map_err(|e| e.to_string())
    }).await.map_err(|e| e.to_string())?
}

/// 读取 .env 文件并设置环境变量
fn load_env_from_file(file_path: &str) -> io::Result<()> {
    let file = File::open(file_path)?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?;

        // 跳过空行或注释行
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }

        // 解析键值对
        if let Some((key, value)) = line.split_once('=') {
            env::set_var(key.trim(), value.trim());
        }
    }
    Ok(())
}

fn main() {
    logger::init_logger().expect("Failed to initialize logger");
    logger::set_log_level(logger::LogLevel::DEBUG);

    tauri::Builder::default()
        .setup(|app| {
            let handle = app.app_handle();  // 获取应用句柄

            // 加载 .env 文件
            if let Err(e) = load_env_from_file("./video.env") {
                log_error!("{}", format!("Failed to load video.env file: {}", e));
            }

            // 初始化数据库
            let conn = db::init_db(&handle).expect("Database initialization failed");   // 初始化数据库
            let db_state = DbState(Arc::new(Mutex::new(conn)));
            app.manage(db_state);

            // 加载设置
            let settings = tauri::async_runtime::block_on(load_settings(handle.clone())).expect("Failed to load settings");
            let app_state = AppState {
                settings: Arc::new(Mutex::new(settings)),
            };
            app.manage(app_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            select_and_scan_folder,
            scan_folder,
            get_cached_videos,
            get_video_duration,
            update_video,
            play_video,
            remove_video,
            save_settings,
            load_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
