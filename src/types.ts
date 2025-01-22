/**
 * 视频信息接口
 */
export interface VideoInfo {
    id: string;
    original_title: string;
    title: string;
    thumbnail: string;
    duration: string;
    path: string;
    category: string;
    description: string;
    create_time: number;
    last_play_time: number;
    play_count: number;
    favorite: boolean;
    tags: string;
    is_series: boolean;
    season: number;
    episode: number;
    episode_title: string;
    episode_overview: string;
}

/**
 * 设置信息接口
 */
export interface Settings {
    player_path: string;
    player_type: string;
    auto_subtitle: boolean;
    subtitle_language: string;
    tmdb_api_key: string;
    auto_tmdb: boolean;
    auto_tmdb_poster: boolean;
}