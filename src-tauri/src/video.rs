// Module: video
use std::{fs};
use std::path::{Path, PathBuf};
use crate::db::VideoInfo;
use crate::{api, metadata};
use crate::{ log_debug, log_error, log_info };
use regex::Regex;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use serde_json::Value;
use std::thread;
use std::time::Duration;

static TV_SHOW_CACHE: Lazy<Mutex<HashMap<String, Value>>> = Lazy::new(|| {
    let cache = Mutex::new(HashMap::new());
    start_cache_cleaner();
    cache
});

fn start_cache_cleaner() {
    thread::spawn(|| {
        loop {
            thread::sleep(Duration::from_secs(3600)); // 每小时清理一次
            clean_cache();
        }
    });
}

fn clean_cache() {
    let mut cache = TV_SHOW_CACHE.lock().unwrap();
    cache.clear();
    log_info!("TV_SHOW_CACHE has been cleared.");
}

/// 获取视频时长
/// 
/// # 参数
/// * `path` - 视频路径
/// # 返回
/// * `Result<String, String>` - 成功返回过滤后的视频时长，失败返回错误信息
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
        r"(?i)\b\d{3,4}p\b", // 分辨率
        r"(?i)\b(?:x264|x265|h264|h265)\b", // 编码格式
        r"(?i)\b(?:bluray|brrip|dvdrip|hdrip|webrip|web-dl|hdcam|hdts|cam|ts|tc|r5|dvdscr|dvdscreener|screener)\b", // 版本
        r"(?i)\b(?:aac|ac3|dts|dd5\.1|mp3|flac|truehd|atmos)\b", // 音频格式
        r"(?i)\b(?:subs|subbed|dubbed|multi|dual)\b", // 字幕/配音
        r"(?i)\b(?:extended|unrated|director(?:'s)? cut|theatrical|special edition|ultimate edition|collector(?:'s)? edition)\b", // 版本类型
        r"(?i)\b(?:repack|proper|rerip|real)\b", // 重新打包
        r"(?i)\b(?:hevc|hdr|uhd|4k|3d|imax)\b", // 其他格式
        r"(?i)\b(?:part|pt|season|s|episode|e|ep|vol|volume|v)\d+\b", // 剧集信息
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
        let extension = Path::new(filename).extension().and_then(|ext| ext.to_str()).unwrap_or("");
        best_result = original_words.into_iter()
            .filter(|w| w.len() > 1 && !w.eq_ignore_ascii_case(extension))
            .take(3)  // 取前三个词
            .collect::<Vec<_>>()
            .join(" ");
    }
    
    log_debug!("Original: {} Cleaned: {}", filename, best_result);
    best_result
}

/// 从 TMDb API 获取视频信息并过滤结果
/// 
/// # 参数
/// * `video_name` - 视频名称
/// 
/// # 返回
/// * `Result<String, String>` - 成功返回过滤后的单个视频信息，失败返回错误信息
pub(crate) async fn fetch_video_info_from_tmdb(video_name: &String, api_key: &String) -> Result<String, String> {
    let cleaned_name = clean_video_name(&video_name);
    log_info!("************Searching for: {}************", cleaned_name);

    let url = format!(
        "https://api.themoviedb.org/3/search/movie?api_key={}&query={}&language=zh-CN",
        api_key,
        cleaned_name
    );

    // 查找最优匹配结果
    let best_match = match_video(&url, &cleaned_name).await?;
    log_info!("Found match: {}", serde_json::to_string_pretty(&best_match).unwrap());

    if best_match.is_empty() || best_match.eq_ignore_ascii_case("null") {
        return Ok(String::new()); // 返回空字符串
    }

    // 解析 best_match 为 serde_json::Value
    let movie: serde_json::Value = serde_json::from_str(&best_match)
    .map_err(|e| e.to_string())?;

    // 检查 movie 是否为 null
    if movie.is_null() {
        log_debug!("Best match is null");
        return Ok(String::new()); // 返回空字符串
    }

    let movie: serde_json::Value = serde_json::from_str(&best_match)
            .map_err(|e| e.to_string())?;

    // 获取电影的类型ID
    let genre_ids = movie.get("genre_ids").and_then(|ids| ids.as_array())
    .map(|ids| ids.iter()
        .filter_map(|id| id.as_i64())
        .collect::<Vec<i64>>())
    .unwrap_or_default();

    // 获取类型名称
    let genres = get_genre_names(&genre_ids, api_key).await?;

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

/// 从 TMDb API 获取视频信息并过滤结果
/// 
/// # 参数
/// * `tv_name` - 视频名称
/// 
/// # 返回
/// * `Result<String, String>` - 成功返回过滤后的单个视频信息，失败返回错误信息
pub(crate) async fn fetch_tv_info_from_tmdb(series_info: &SeriesInfo, api_key: &String) -> Result<String, String> {
    let cleaned_name = &series_info.series_title.replace(".", " "); //clean_video_name(&series_info.series_title);
    log_info!("************Searching for: {}************", cleaned_name);

    let mut series: Option<Value> = None;
    let mut season_info: Option<Value> = None;

    // 检查缓存
    {
        let cache = TV_SHOW_CACHE.lock().unwrap();
        if let Some(cached_info) = cache.get(cleaned_name.as_str()) {
            log_info!("Cache hit for: {}", cleaned_name);
            // 访问缓存中的值
            series = cached_info.get("series").cloned();
            season_info = cached_info.get("season_info").cloned();
        }
    };

    if series.is_none() {
        let url = format!(
            "https://api.themoviedb.org/3/search/tv?api_key={}&query={}&language=zh-CN",
            api_key,
            cleaned_name
        );
    
        let best_match = match_video(&url, &cleaned_name).await?;

        if best_match.is_empty() || best_match.eq_ignore_ascii_case("null") {
            log_info!("Found match: {}", serde_json::to_string_pretty(&best_match).unwrap());
            return Ok(String::new());
        }
        series = serde_json::from_str(&best_match).map_err(|e| e.to_string())?;
    }
    let series = series.as_ref().ok_or_else(|| "Series not found".to_string())?;

    if season_info.is_none() {
        // 系列ID
        let series_id = series.get("id").and_then(|id| id.as_i64()).ok_or("Series ID not found")?;

        let url = format!(
            "https://api.themoviedb.org/3/tv/{}/season/{}?api_key={}&language=zh-CN",
            series_id,
            series_info.season,
            api_key
        );
        log_debug!("API URL: {}", url);
        
        // Season 详细信息
        let season_info_str = api::get_data(&url).await.map_err(|e| e.to_string())?;
        season_info = Some(serde_json::from_str::<Value>(&season_info_str).map_err(|e| {
            log_error!("Failed to parse Season info: {}", e);
            "Failed to parse Season info".to_string()
        })?);
    }
    let season_info = season_info.as_ref().ok_or_else(|| "Season not found".to_string())?;


    // Episode 详细信息
    let episode_info = get_episode_info(&season_info, series_info.episode as u32).cloned();
    let episode_info = episode_info.as_ref().ok_or_else(|| "Episode not found".to_string())?;
    
    // 获取电视剧的类型ID
    let genre_ids = series.get("genre_ids").and_then(|ids| ids.as_array())
                                .map(|ids| ids.iter()
                                    .filter_map(|id| id.as_i64())
                                    .collect::<Vec<i64>>())
                                .unwrap_or_default();
    // 获取类型名称
    let genres = get_genre_names(&genre_ids, api_key).await?;

    // 缓存结果
    {
        let mut cache = TV_SHOW_CACHE.lock().unwrap();
        let cache_value = serde_json::json!({
            "series": series,
            "season_info": season_info,
        });
        cache.insert(cleaned_name.clone(), cache_value);
    }

    // 构建我们需要的信息
    let filtered_info = serde_json::json!({
        "title": episode_info.get("name").and_then(|t| t.as_str()).unwrap_or(""),
        "original_title": series.get("original_name").and_then(|t| t.as_str()).unwrap_or(""),
        "overview": series.get("overview").and_then(|t| t.as_str()).unwrap_or(""),
        "release_date": series.get("release_date").and_then(|t| t.as_str()).unwrap_or(""),
        "poster_path": series.get("poster_path").and_then(|t| t.as_str())
            .map(|path| format!("https://image.tmdb.org/t/p/w500{}", path))
            .unwrap_or_default(),
        "vote_average": season_info.get("vote_average").and_then(|t| t.as_f64()).unwrap_or(0.0),
        "genres": genres,
        "series_title": series.get("name").and_then(|t| t.as_str()).unwrap_or(""),
        "episode_overview": episode_info.get("overview").and_then(|t| t.as_str()).unwrap_or("")
    });
    
    return Ok(serde_json::to_string(&filtered_info).unwrap());
}

fn get_episode_info(season_info: &serde_json::Value, episode_number: u32) -> Option<&serde_json::Value> {
    println!("episode_number: {}", &episode_number);

    // 获取 episodes 数组
    let episodes = season_info.get("episodes");

    // 转换为数组
    let episodes_array = episodes.and_then(|episodes| episodes.as_array());

    // 查找匹配的 episode
    let episode = episodes_array.and_then(|episodes_array| {
        episodes_array.iter().find(|episode| {
            let episode_num = episode.get("episode_number").and_then(|num| num.as_u64());
            episode_num.map(|num| num == episode_number as u64).unwrap_or(false)
        })
    });

    episode
}

/// 从 TMDB API 获取视频信息，根据视频名过滤结果
/// # 参数
/// * `url` - 请求URL
/// * `video_name` - 视频名
/// 
/// # 返回
/// * `Result<String, String>` - 成功返回过滤后的单个视频信息，失败返回错误信息
async fn match_video(url: &String, video_name: &String) -> Result<String, String> {
    log_debug!("API URL: {}", url);
    match api::get_data(url).await {
        Ok(response) => {
            let json: serde_json::Value = serde_json::from_str(&response)
            .map_err(|e| e.to_string())?;
            log_debug!("API Response: {}", response);
            // 获取结果数组
            if let Some(results) = json.get("results").and_then(|v| v.as_array()) {
                // 查找最匹配的结果
                let best_match = results.iter().find(|tv| {
                    // 获取标题（优先使用中文标题）
                    let title = tv.get("title").and_then(|t| t.as_str()).unwrap_or("");
                    let original_title = tv.get("original_title").and_then(|t| t.as_str()).unwrap_or("");

                    // 优先匹配同名
                    title.eq_ignore_ascii_case(&video_name) || 
                    original_title.eq_ignore_ascii_case(&video_name)
                }).or_else(|| {
                    // 如果没有找到同名的，则匹配包含的名称
                    results.iter().find(|tv| {
                        let title = tv.get("title").and_then(|t| t.as_str()).unwrap_or("");
                        let original_title = tv.get("original_title").and_then(|t| t.as_str()).unwrap_or("");

                        title.to_lowercase().contains(&video_name.to_lowercase()) ||
                        original_title.to_lowercase().contains(&video_name.to_lowercase())
                    })
                }).or_else(|| results.first()); // 如果没有找到匹配的，则返回第一个结果
                return Ok(serde_json::to_string(&best_match).unwrap_or_else(|_| "No matching movie found".to_string()));
            }
            
            log_debug!("{} :No matching movie found", video_name);
            return Ok(String::new());
        },
        Err(e) => {
            log_error!("API Error: {}", e);
            Err(e.to_string())
        }
    }
}

// 获取类型名称的辅助函数
pub(crate) async fn get_genre_names(genre_ids: &[i64], api_key: &String) -> Result<String, String> {

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