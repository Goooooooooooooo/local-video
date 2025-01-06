use crate::{log_info, log_debug};

use rusqlite::{params, Connection, OptionalExtension, Result};
use std::fs;
use std::sync::{Mutex, Arc};
use tauri::{AppHandle, Manager};
use serde::{Serialize, Deserialize};

/// 视频信息结构体
/// 
/// 存储视频的基本信息，包括ID、标题、缩略图、时长等
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoInfo {
    /// 视频唯一标识符，使用文件路径的MD5值
    pub id: String,
    /// 视频标题，默认使用文件名
    pub title: String,
    /// 视频标题（中文），可选
    pub title_cn: String,
    /// 缩略图URL
    pub thumbnail: String,
    /// 视频时长
    pub duration: String,
    /// 视频文件路径
    pub path: String,
    /// 视频分类
    pub category: String,
    /// 视频描述
    pub description: String,
    /// 创建时间（Unix时间戳）
    pub create_time: i64,
    /// 最后播放时间（Unix时间戳）
    pub last_play_time: i64,
    /// 播放次数
    pub play_count: i32,
    /// 是否收藏
    pub favorite: bool,
    /// 标签（逗号分隔的字符串）
    pub tags: String,
    /// 是否为剧集
    pub is_series: bool,
    /// 系列名称（用于电视剧）
    pub series_title: String,
    /// 季数
    pub season: i32,
    /// 集数
    pub episode: i32,
}

/// 数据库连接状态
/// 
/// 使用互斥锁包装SQLite连接，确保线程安全
pub struct DbState(pub Arc<Mutex<Connection>>);

/// 初始化数据库
/// 
/// ## 参数
/// * `app_handle` - Tauri应用句柄，用于获取应用数据目录
/// 
/// ## 返回
/// * `Result<Connection>` - 成功返回数据库连接，失败返回错误
/// 
/// ## 示例
/// ```rust
/// let conn = init_db(&app_handle).expect("Database initialization failed");
/// ```
pub fn init_db(app_handle: &AppHandle) -> Result<Connection> {
    log_info!("Initializing database...");
    // 打印调用栈
    log_debug!("Call stack:\n{:?}", std::backtrace::Backtrace::capture());
    
    let app_dir = app_handle.path().app_data_dir().unwrap();
    log_debug!("Database path: {}", app_dir.join("videos.db").display());
    fs::create_dir_all(&app_dir).unwrap();
    let db_path = app_dir.join("videos.db");
    
    let conn = Connection::open(db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS videos (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            title_cn TEXT NOT NULL,
            thumbnail TEXT NOT NULL,
            duration TEXT NOT NULL,
            path TEXT NOT NULL,
            category TEXT NOT NULL,
            description TEXT NOT NULL,
            create_time INTEGER NOT NULL,
            last_play_time INTEGER NOT NULL,
            play_count INTEGER NOT NULL,
            favorite BOOLEAN NOT NULL,
            tags TEXT NOT NULL,
            is_series BOOLEAN NOT NULL DEFAULT 0,
            series_title TEXT NOT NULL DEFAULT '',
            season INTEGER NOT NULL DEFAULT 1,
            episode INTEGER NOT NULL DEFAULT 1
        )",
        [],
    )?;
    
    Ok(conn)
}

/// 通用执行查询方法
// fn execute_query(conn: &Connection, query: &str, params: &[&dyn rusqlite::ToSql]) -> Result<()> {
//     conn.execute(query, params)?; // 执行无返回值的SQL查询
//     Ok(())
// }

/// 通用单行查询方法
fn fetch_single_row<T>(
    conn: &Connection,
    query: &str,
    params: &[&dyn rusqlite::ToSql],
    mapper: impl Fn(&rusqlite::Row) -> Result<T, rusqlite::Error>,
) -> Result<Option<T>> {
    conn.query_row(query, params, |row| mapper(row)).optional() // 处理单行查询结果
}

/// 向数据库插入视频信息
/// 
/// # 参数
/// * `conn` - 数据库连接
/// * `video` - 要插入的视频信息
/// 
/// # 返回
/// * `Result<(), rusqlite::Error>` - 成功返回Ok(()), 失败返回错误
pub fn insert_video(conn: &Connection, video: &VideoInfo) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO videos (
            id, title, title_cn, thumbnail, duration, path, category, description,
            create_time, last_play_time, play_count, favorite, tags,
            is_series, series_title, season, episode
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            video.id,
            video.title,
            video.title_cn,
            video.thumbnail,
            video.duration,
            video.path,
            video.category,
            video.description,
            video.create_time,
            video.last_play_time,
            video.play_count,
            video.favorite,
            video.tags,
            video.is_series,
            video.series_title,
            video.season,
            video.episode,
        ],
    )?;
    log_debug!("Inserted video: {:?}", video);
    Ok(())
}

/// 检查视频是否存在
/// 
/// # 参数
/// * `conn` - 数据库连接
/// * `id` - 视频ID
/// 
/// # 返回
/// * `bool` - 存在返回true，不存在返回false
pub fn video_exists(conn: &Connection, id: &str) -> bool {
    // conn.query_row("SELECT 1 FROM videos WHERE id = ?1", params![id], |_| Ok(true)).unwrap_or(false);
    fetch_single_row(&conn, "SELECT 1 FROM videos WHERE id = ?", &[&id], |_| Ok(()))
        .map(|opt| opt.is_some())
        .unwrap_or(false) // 查询是否存在记录
}

/// 获取所有视频
/// 
/// # 参数
/// * `conn` - 数据库连接
/// 
/// # 返回
/// * `Result<Vec<VideoInfo>, rusqlite::Error>` - 成功返回视频列表，失败返回错误
pub fn get_all_videos(conn: &Connection) -> Result<Vec<VideoInfo>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, title, title_cn, thumbnail, duration, path, category, description,
         create_time, last_play_time, play_count, favorite, tags,
         is_series, series_title, season, episode
         FROM videos"
    )?;

    let videos = stmt.query_map([], |row| {
        Ok(VideoInfo {
            id: row.get(0)?,
            title: row.get(1)?,
            title_cn: row.get(2)?,
            thumbnail: row.get(3)?,
            duration: row.get(4)?,
            path: row.get(5)?,
            category: row.get(6)?,
            description: row.get(7)?,
            create_time: row.get(8)?,
            last_play_time: row.get(9)?,
            play_count: row.get(10)?,
            favorite: row.get(11)?,
            tags: row.get(12)?,
            is_series: row.get(13)?,
            series_title: row.get(14)?,
            season: row.get(15)?,
            episode: row.get(16)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(videos)
}

pub fn delete_video(conn: &Connection, id: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM videos WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}