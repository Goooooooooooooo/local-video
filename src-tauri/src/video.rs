// Module: video
use std::{env, fs};
use std::path::{Path, PathBuf};
use crate::db::VideoInfo;
use crate::{api, metadata};
use crate::{ log_debug, log_error, log_info };
use regex::Regex;

pub(crate) fn get_duration(path: &str) -> Result<String, String> {
    log_debug!("Getting video duration for: {}", path);
    let duration = match metadata::mkv_metadata(path) {
        Ok(metadata) => {
            println!("metadata: {:?}", metadata);
            metadata.video_duration_seconds
        },
        Err(e) => {
            log_error!("Failed to get video duration: {}", e);
            0.0
        }
    };

    let hours = duration as u64 / 3600;
    let minutes = duration as u64 % 3600 / 60;
    let seconds = duration as u64 % 60;
    log_debug!("Duration: {:02}:{:02}:{:02}", hours, minutes, seconds);
    Ok(format!("{:02}:{:02}:{:02}", hours, minutes, seconds))

}

/// 查找字幕文件
pub(crate) fn find_subtitles(video: &VideoInfo) -> Result<String, String> {
    log_debug!("Getting subtitle for: {}", video.path);
    let current_path = Path::new(&video.path);
    let current_dir = current_path.parent().expect("Failed to get parent directory");
    log_debug!("Current directory: {}", current_dir.display());
    let subtitle_path = current_dir.join("字幕");

    // 遍历目录下的文件
    let mut subtitles = Vec::new();
    if let Ok(entries) = fs::read_dir(subtitle_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                // 匹配字幕扩展名
                if let Some(ext) = path.extension() {
                    if ext == "srt" || ext == "ass" || ext == "vtt" {
                        subtitles.push(path);
                    }
                }
            }
        }
    }

    if let Some(best_subtitle) = choose_best_subtitle(video, subtitles) {
        log_debug!("Best subtitle: {}", best_subtitle.display());
        Ok(best_subtitle.to_string_lossy().to_string())
    } else {
        log_debug!("No suitable subtitles found.");
        Err("No suitable subtitles found.".to_string())
    }

}

/// 根据优先级选择最佳字幕文件
/// 优先级规则：
/// 1. 文件名与视频文件名完全匹配（不含扩展名）。
/// 2. 包含指定语言标记（如 .zh.srt, .chs.srt, .cn.srt）。
fn choose_best_subtitle(video: &VideoInfo, subtitles: Vec<PathBuf>) -> Option<PathBuf> {
    let video_stem = Path::new(&video.path).file_stem()?.to_string_lossy();
    let language_keywords = ["zh", "chs", "cht", "cn", "chinese", "chr", "简体", "简中", "繁中"];

    let series_info = parse_series_info(&video_stem);
    let mut series_pattern = String::new();
    if series_info.is_series {
        series_pattern = format!(
            "S{:02}E{:02}",
            series_info.season,
            series_info.episode,
        );
    }
    log_debug!("video_stem: {:?}", video_stem);
    log_debug!("episode_pattern: {:?}", series_pattern);

    subtitles.into_iter().max_by_key(|subtitle| {
        if let Some(subtitle_stem) = subtitle.file_stem().and_then(|s| s.to_str()) {
            if subtitle_stem == video_stem {
                3 // 完全匹配得分最高
            } else {
                if series_info.is_series {
                    if subtitle_stem.contains(&series_pattern) 
                    && language_keywords.iter().any(|&keyword| subtitle_stem.to_ascii_lowercase().contains(keyword)) {
                        log_debug!("subtitle_stem 1: {}", subtitle_stem);
                        2 // 包含剧集编号和语言标记得分次之
                    } else {
                        1
                    }
                } else { 
                    if language_keywords.iter().any(|&keyword| subtitle_stem.to_ascii_lowercase().contains(keyword)) {
                        log_debug!("subtitle_stem 2: {}", subtitle_stem);
                        2 // 包含语言标记得分次之
                    } else {
                        log_debug!("subtitle_stem 3: {}", subtitle_stem.to_ascii_lowercase());
                        1
                    }
                }
            }
        } else {
            0 // 无法获取文件名时最低优先级
        }
    })
}

/// 清理视频文件名以获得更好的搜索结果
pub(crate) fn clean_video_name(filename: &str) -> String {
    // 移除扩展名
    let name = filename.strip_suffix(std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or(""))
        .unwrap_or(filename)
        .trim_end_matches('.');
    log_debug!("Processing name: {}", name);
    // 1. 首先尝试提取年份前的内容（通常是电影名称）
    if let Some(year_match) = regex::Regex::new(r"^(.*?)\b(19|20)\d{2}\b")
        .unwrap()
        .captures(name) {
        if let Some(title) = year_match.get(1) {
            let mut cleaned = title.as_str().to_string();
            // 清理分隔符
            cleaned = cleaned.replace('.', " ")
                           .replace('_', " ")
                           .replace('-', " ");
            // 清理多余空格
            cleaned = cleaned.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            if !cleaned.is_empty() {
                return cleaned;
            }
        }
    }
    
    // 2. 如果没有找到年份，则进行常规清理
    let cleaned = name.to_string();
    
    // 定义需要移除的模式
    let patterns = [
        // ... (之前的所有模式保持不变)
    ];
    
    // 3. 保持清理过程，但添加结果验证
    let original_words: Vec<&str> = name.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
        
    let mut best_result = cleaned.clone();
    let mut max_words = 0;
    
    // 逐步应用模式，每次检查结果
    for pattern in patterns {
        let temp_cleaned = regex::Regex::new(pattern)
            .unwrap()
            .replace_all(&cleaned, " ")
            .to_string();
            
        let words: Vec<&str> = temp_cleaned.split_whitespace()
            .filter(|w| w.len() > 1)  // 忽略单字符
            .collect();
            
        // 如果清理后还有有意义的词，更新结果
        if words.len() > max_words {
            max_words = words.len();
            best_result = words.join(" ");
        }
    }
    
    // 4. 如果清理后结果为空或只剩文件扩展名，回退到原始文件名的首个有意义词组
    if best_result.is_empty() || best_result.len() < 3 {
        best_result = original_words.into_iter()
            .filter(|w| w.len() > 1 && !w.eq_ignore_ascii_case("mkv"))
            .take(3)  // 取前三个词
            .collect::<Vec<_>>()
            .join(" ");
    }
    
    log_debug!("Original: {}\nCleaned: {}", filename, best_result);
    best_result
}

/// 从 TMDb API 获取视频信息并过滤结果
/// 
/// # 参数
/// * `video_name` - 视频名称
/// 
/// # 返回
/// * `Result<String, String>` - 成功返回过滤后的单个视频信息，失败返回错误信息
pub(crate) async fn fetch_video_info_from_tmdb(video_name: &String) -> Result<String, String> {
    let cleaned_name = clean_video_name(&video_name);
    log_info!("************Searching for: {}************", cleaned_name);

    let api_key = env::var("TMDB_API_KEY").unwrap_or_else(|_| String::from("default_key"));

    let url = format!(
        "https://api.themoviedb.org/3/search/movie?api_key={}&query={}&language=zh-CN",
        api_key,
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
                } else {
                    let filtered_info = serde_json::json!({
                        "title": cleaned_name,
                        "original_title": cleaned_name,
                        "overview": "未找到匹配的电影信息",
                        "release_date": "",
                        "poster_path": "/assets/no-poster.png",
                        "vote_average": 0.0,
                        "genres": "未分类",
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
pub(crate) async fn get_genre_names(genre_ids: &[i64]) -> Result<String, String> {

    let api_key = env::var("TMDB_API_KEY").unwrap_or_else(|_| String::from("default_key"));

    let url = format!(
        "https://api.themoviedb.org/3/genre/movie/list?api_key={}&language=zh-CN",
        api_key
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

#[derive(Debug)]
pub struct SeriesInfo {
    pub series_title: String,
    pub season: i32,
    pub episode: i32,
    pub is_series: bool,
}

pub fn parse_series_info(filename: &str) -> SeriesInfo {
    // 常见的剧集命名模式
    let patterns = [
        // S01E01 格式
        r"(?i)(.+?)[\s.]*S(\d{1,2})E(\d{1,2})",
        // 第1季第1集 格式
        r"(.+?)第(\d{1,2})季第(\d{1,2})集",
        // 第01集 格式（假定为第1季）
        r"(.+?)第(\d{1,2})集",
        // E01 格式（假定为第1季）
        r"(?i)(.+?)[\s.]*E(\d{1,2})",
    ];

    for pattern in patterns {
        if let Some(caps) = Regex::new(pattern).unwrap().captures(filename) {
            match pattern {
                r"(?i)(.+?)[\s.]*S(\d{1,2})E(\d{1,2})" => {
                    return SeriesInfo {
                        series_title: caps.get(1).unwrap().as_str().trim().to_string(),
                        season: caps.get(2).unwrap().as_str().parse().unwrap_or(1),
                        episode: caps.get(3).unwrap().as_str().parse().unwrap_or(1),
                        is_series: true,
                    };
                }
                r"(.+?)第(\d{1,2})季第(\d{1,2})集" => {
                    return SeriesInfo {
                        series_title: caps.get(1).unwrap().as_str().trim().to_string(),
                        season: caps.get(2).unwrap().as_str().parse().unwrap_or(1),
                        episode: caps.get(3).unwrap().as_str().parse().unwrap_or(1),
                        is_series: true,
                    };
                }
                r"(.+?)第(\d{1,2})集" => {
                    return SeriesInfo {
                        series_title: caps.get(1).unwrap().as_str().trim().to_string(),
                        season: 1,
                        episode: caps.get(2).unwrap().as_str().parse().unwrap_or(1),
                        is_series: true,
                    };
                }
                r"(?i)(.+?)[\s.]*E(\d{1,2})" => {
                    return SeriesInfo {
                        series_title: caps.get(1).unwrap().as_str().trim().to_string(),
                        season: 1,
                        episode: caps.get(2).unwrap().as_str().parse().unwrap_or(1),
                        is_series: true,
                    };
                }
                _ => {}
            }
        }
    }

    // 如果没有匹配到任何模式，返回默认值
    SeriesInfo {
        series_title: String::new(),
        season: 1,
        episode: 1,
        is_series: false,
    }
}



mod tests {

    use super::*;

    #[test]
    fn match_sutitles() {
        let temp_file_path = "C:\\Users\\yzok0\\Videos\\Transformers.One.2024.HDR.2160p.WEB.h265-ETHEL[TGx]\\Transformers.One.2024.HDR.2160p.WEB.h265-ETHEL.mkv";
        let subtitle_stem = "Transformers.One.xx.2024.HDR.2160p.WEB.h265-ETHEL.简体.srt";
        let episode_pattern = regex::Regex::new(r"S\d{2}E\d{2}").ok(); // 匹配剧集编号 SxxExx
        let language_keywords = ["zh", "chs", "cn", "cht", "chinese", "chr", "简体", "简中", "繁中"];
        let _language_pattern = regex::Regex::new(&format!(r"({})", language_keywords.join("|"))).ok(); // 匹配语言关键字    
        
        let series_info = parse_series_info(temp_file_path);
        if series_info.is_series {
            println!("Series: {:?}", series_info.is_series);
            let series_pattern = format!(
                "[\\s.]*S{:02}E{:02}*{}",
                series_info.season,
                series_info.episode,
                language_keywords.join("|")
            );
            println!("Series pattern: {}", series_pattern);
            let re = Regex::new(&series_pattern).unwrap();
            if re.is_match(subtitle_stem) {
                println!("Matched: {}", subtitle_stem);
            }
        }

        if let Some(ep_pattern) = &_language_pattern {
            if ep_pattern.is_match(&subtitle_stem) {
                println!("Episode: {:?}", ep_pattern.captures(&subtitle_stem));
            }
        }
    }
}