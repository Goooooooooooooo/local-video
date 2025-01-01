#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod api;
mod video;
mod logger;

use walkdir::WalkDir;
use tauri::{Manager, State};
use db::{DbState, VideoInfo, init_db, insert_video, video_exists, get_all_videos};
use std::sync::{Mutex, Arc};
use video::{clean_video_name, parse_series_info};
use std::process::Command;
use serde::{Deserialize, Serialize};
use std::fs;

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
                VIDEO_EXTENSIONS.contains(&ext.to_string_lossy().to_lowercase().as_str())
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
                                log_debug!("video: {:?}", video);
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

/// 从 TMDb API 获取视频信息并过滤结果
/// 
/// # 参数
/// * `video_name` - 视频名称
/// 
/// # 返回
/// * `Result<String, String>` - 成功返回过滤后的单个视频信息，失败返回错误信息
#[tauri::command]
async fn fetch_video_info_from_tmdb(video_name: &String) -> Result<String, String> {
    let cleaned_name = clean_video_name(&video_name);

    log_info!("************Searching for: {}************", cleaned_name);
    let url = format!(
        "https://api.themoviedb.org/3/search/movie?api_key={}&query={}&language=zh-CN",
        "c1aca3c36b4dbc238a502753b4619473",
        cleaned_name
    );
    log_debug!("API URL: {}", url);
    match api::get_data(&url).await {
        Ok(response) => {
            let json: serde_json::Value = serde_json::from_str(&response)
                .map_err(|e| e.to_string())?;
            log_debug!("API Response: {}", response);
            // 获取结果数组
            if let Some(results) = json.get("results").and_then(|v| v.as_array()) {
                // 查找最匹配的结果
                let best_match = results.iter().find(|movie| {
                    // 获取标题（优先使用中文标题）
                    let title = movie.get("title").and_then(|t| t.as_str()).unwrap_or("");
                    let original_title = movie.get("original_title").and_then(|t| t.as_str()).unwrap_or("");
                    
                    // 简单的相似度匹配（可以根据需要调整匹配逻辑）
                    title.to_lowercase().contains(&cleaned_name.to_lowercase()) ||
                    original_title.to_lowercase().contains(&cleaned_name.to_lowercase())
                }).or_else(|| results.first()); // 如果没有找到匹配的，则返回第一个结果

                if let Some(movie) = best_match {
                    log_info!("Found match: {}", serde_json::to_string_pretty(&movie).unwrap());

                    // 获取电影的类型ID
                    let genre_ids = movie.get("genre_ids")
                    .and_then(|ids| ids.as_array())
                    .map(|ids| ids.iter()
                        .filter_map(|id| id.as_i64())
                        .collect::<Vec<i64>>())
                    .unwrap_or_default();

                    // 获取类型名称
                    let genres = get_genre_names(&genre_ids).await?;

                    // 构建我们需要的信息
                    let filtered_info = serde_json::json!({
                        "title": movie.get("title").and_then(|t| t.as_str()).unwrap_or(""),
                        "original_title": movie.get("original_title").and_then(|t| t.as_str()).unwrap_or(""),
                        "overview": movie.get("overview").and_then(|t| t.as_str()).unwrap_or(""),
                        "release_date": movie.get("release_date").and_then(|t| t.as_str()).unwrap_or(""),
                        "poster_path": movie.get("poster_path").and_then(|t| t.as_str())
                            .map(|path| format!("https://image.tmdb.org/t/p/w500{}", path))
                            .unwrap_or_default(),
                        "vote_average": movie.get("vote_average").and_then(|t| t.as_f64()).unwrap_or(0.0),
                        "genres": genres,
                    });
                    
                    return Ok(serde_json::to_string(&filtered_info).unwrap());
                }
            }
            
            Err("No matching movie found".to_string())
        },
        Err(e) => {
            log_error!("API Error: {}", e);
            Err(e.to_string())
        }
    }
}

// 获取类型名称的辅助函数
async fn get_genre_names(genre_ids: &[i64]) -> Result<String, String> {
    let url = format!(
        "https://api.themoviedb.org/3/genre/movie/list?api_key={}&language=zh-CN",
        "c1aca3c36b4dbc238a502753b4619473"
    );
    
    match api::get_data(&url).await {
        Ok(response) => {
            let json: serde_json::Value = serde_json::from_str(&response)
                .map_err(|e| e.to_string())?;
            
            if let Some(genres) = json.get("genres").and_then(|v| v.as_array()) {
                let genre_names: Vec<String> = genres.iter()
                    .filter(|genre| {
                        genre.get("id")
                            .and_then(|id| id.as_i64())
                            .map(|id| genre_ids.contains(&id))
                            .unwrap_or(false)
                    })
                    .filter_map(|genre| {
                        genre.get("name")
                            .and_then(|name| name.as_str())
                            .map(String::from)
                    })
                    .collect();
                
                Ok(genre_names.join("、"))
            } else {
                Ok("未分类".to_string())
            }
        },
        Err(e) => Err(e.to_string())
    }
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

fn main() {
    logger::init_logger().expect("Failed to initialize logger");
    logger::set_log_level(logger::LogLevel::DEBUG);
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.app_handle();
            let conn = init_db(&handle).expect("Database initialization failed");
            app.manage(DbState(Arc::new(Mutex::new(conn))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            select_and_scan_folder,
            scan_folder,
            get_cached_videos,
            play_video,
            save_settings,
            load_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
