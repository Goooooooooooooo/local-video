use crate::log_debug;
use regex::Regex;

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