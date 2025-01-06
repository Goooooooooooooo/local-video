#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod api;
mod video;
mod logger;

use walkdir::WalkDir;
use tauri::{Manager, State};
use db::{delete_video, get_all_videos, init_db, insert_video, video_exists, DbState, VideoInfo};
use std::{
    env, 
    fs,
    fs::File, 
    io::{self, BufRead},
    sync::{Mutex, Arc}, 
    process::Command};
use video::{fetch_video_info_from_tmdb, parse_series_info};
use serde::{Deserialize, Serialize};

// 导出日志宏
pub use crate::logger::{log_error, log_info, log_debug};

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "mov"];

#[derive(Serialize, Deserialize, Debug)]
struct Settings {
    player_path: Option<String>,
    player_type: Option<String>,
}

#[tauri::command]
async fn scan_folder(path: String, db: State<'_, DbState>) -> Result<Vec<VideoInfo>, String> {
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
            video_exists(&conn, &id_clone)
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
            let file_name_clone = file_name.clone();
            
            // 解析剧集信息
            let series_info = parse_series_info(&file_name);
            let search_name = if series_info.is_series {
                &series_info.series_title
            } else {
                &file_name
            };
            
            // 获取 TMDb 信息
            match fetch_video_info_from_tmdb(search_name).await {
                Ok(video_info_str) => {
                    log_debug!("video_info_str: {}", video_info_str);
                    match serde_json::from_str::<serde_json::Value>(&video_info_str) {
                        Ok(video_info) => {
                            let db_clone = db.clone();
                            let id_clone = id.clone();
                            let new_videos = new_videos.clone();
                            

                            if let Err(e) = tokio::task::spawn_blocking(move || {
                                let conn = db_clone.lock().unwrap();
                                let video = VideoInfo {
                                    id: id_clone,
                                    title: video_info.get("original_title").and_then(|v| v.as_str()).unwrap_or(&file_name).to_string(),
                                    title_cn: video_info.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                    thumbnail: video_info.get("poster_path").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                    duration: "".to_string(),
                                    path: path.to_string_lossy().to_string(),
                                    category: video_info.get("genres").and_then(|v| v.as_str()).unwrap_or("未分类").to_string(),
                                    description: video_info.get("overview").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                    create_time: chrono::Utc::now().timestamp(),
                                    last_play_time: 0,
                                    play_count: 0,
                                    favorite: false,
                                    tags: String::new(),
                                    is_series: series_info.is_series,
                                    series_title: series_info.series_title,
                                    season: series_info.season,
                                    episode: series_info.episode,
                                };

                                insert_video(&conn, &video).map_err(|e| e.to_string())?;
                                new_videos.lock().unwrap().push(video);
                                Ok::<_, String>(())
                            }).await {
                                log_error!("Failed to process video {}: {}", file_name_clone, e);
                                continue;
                            }
                        }
                        Err(e) => {
                            log_error!("Failed to parse video info for {}: {}", file_name, e);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    log_error!("Failed to fetch TMDb info for {}: {}", search_name, e);
                    continue;
                }
            }
        }
    }

    // 最后获取所有视频
    tokio::task::spawn_blocking(move || {
        let conn = db.lock().unwrap();
        get_all_videos(&conn).map_err(|e| e.to_string())
    }).await.unwrap()
}

#[tauri::command]
async fn select_and_scan_folder(db: State<'_, DbState>) -> Result<Vec<VideoInfo>, String> {
    if let Some(path) = rfd::FileDialog::new().pick_folder() {
        scan_folder(path.to_string_lossy().to_string(), db).await
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
    get_all_videos(&conn).map_err(|e| e.to_string())
}



#[tauri::command]
async fn play_video(path: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    let settings = load_settings(app_handle.clone()).await?;
    
    match settings.player_path {
        Some(player_path) if !player_path.is_empty() => {
            Command::new(player_path)
                .arg(&path)
                .spawn()
                .map_err(|e| e.to_string())?;
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
        })
    }
}

#[tauri::command]
fn remove_video(db: State<'_, DbState>, id: String) -> Result<(), String> {
    let conn = match db.0.try_lock() {
        Ok(lock) => lock,
        Err(_) => return Err("Failed to acquire database lock".to_string()),
    };
    delete_video(&conn, &id).map_err(|e| e.to_string())
}

// fn get_video_duration(path: &str) -> Result<String, String> {
    // let path = Path::new(path);
    // let decoder = Decoder::new(path).map_err(|e| e.to_string())?;
    
    // let duration = decoder.duration().map_err(|e| e.to_string())?;
    // let seconds = duration.as_secs_f64();
    // let hours = (seconds / 3600.0) as u64;
    // let minutes = ((seconds % 3600.0) / 60.0) as u64;
    // let secs = (seconds % 60.0) as u64;
    
    // if hours > 0 {
    //     Ok(format!("{:02}:{:02}:{:02}", hours, minutes, secs))
    // } else {
    //     Ok(format!("{:02}:{:02}", minutes, secs))
    // }
// }

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
    load_env_from_file("./video.env").expect("Failed to load .env file");  // 加载 .env 文件
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.app_handle();  // 获取应用句柄
            let conn = init_db(&handle).expect("Database initialization failed");   // 初始化数据库
            app.manage(DbState(Arc::new(Mutex::new(conn))));    // 将数据库连接状态传递给应用
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            select_and_scan_folder,
            scan_folder,
            get_cached_videos,
            play_video,
            remove_video,
            save_settings,
            load_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
