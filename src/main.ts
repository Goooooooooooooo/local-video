import { invoke } from "@tauri-apps/api/core";
import SimpleAlert from './simplealert';

/**
 * 视频信息接口
 */
interface VideoInfo {
    id: string;
    title: string;
    title_cn: string;
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
    series_title: string;
    season: number;
    episode: number;
}

/**
 * 设置信息接口
 */
interface Settings {
    player_path: string;
    player_type: string;
    auto_subtitle: boolean;
    subtitle_language: string;
    tmdb_api_key: string;
    auto_tmdb: boolean;
}

/** 全局变量：视频列表 */
let _videos: VideoInfo[] = [];
/** 全局变量：当前激活的筛选关键字 */
let _active: string = 'all';
let _simpleAlert: SimpleAlert;

/**
 * 初始化调整大小功能
 * @returns 
 */
function initializeResizer(): void {
  const resizer = document.getElementById('resizer');
  const sidebar = document.querySelector('.sidebar') as HTMLElement;
  console.log(sidebar);
  const root = document.documentElement;

  if (!resizer) return;

  let isResizing = false;

  resizer.addEventListener('mousedown', (e) => {
    console.log(e);
    isResizing = true;
    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', () => {
      isResizing = false;
      document.removeEventListener('mousemove', handleMouseMove);
    });
  });

  const handleMouseMove = (e: MouseEvent) => {
    if (!isResizing) return;
    
    const newWidth = e.clientX;
    if (newWidth > 100 && newWidth < window.innerWidth / 2) {
      root.style.setProperty('--sidebar-width', `${newWidth}px`);
      resizer.style.left = `${newWidth}px`;
    }
  }
}

/**
 * 创建视频卡片
 * @param video 视频信息
 * @returns 
 */
function createVideoCard(video: VideoInfo): HTMLElement {
  const card = document.createElement('div');
  card.className = 'video-card';
  
  const title = video.is_series 
        ? `${video.series_title} S${video.season.toString().padStart(2, '0')}E${video.episode.toString().padStart(2, '0')}`
        : video.title_cn || video.title;
  
  card.id = video.id;
  card.innerHTML = `
    <div class="video-thumbnail">
      <img src="${video.thumbnail}" alt="${title}">
      <div class="card-play-button"></div>
    </div>
    <div class="video-info">
      <h3 class="video-title">${title}</h3>
      <div class="video-metadata">
        <span class="video-duration">${video.duration}</span>
        <span class="video-category">${video.tags}</span>
      </div>
    </div>
    <div class="close-button" title="Close">×</div>
  `;

  const cardCloseButton = card.querySelector('.close-button');
  cardCloseButton?.addEventListener('click', async (e) => {
    e.stopPropagation();
    await invoke('remove_video', { id: video.id });
    card.remove();
    _videos = _videos.filter(v => v.id !== video.id);
  });

  // 播放按钮点击事件
  const cardPlayButton = card.querySelector('.card-play-button');
  cardPlayButton?.addEventListener('click', async (e) => {
    e.stopPropagation(); // 阻止事件冒泡
    try {
      await invoke('play_video', { video: video });
    } catch (error) {
      console.error('Error playing video:', error);
      _simpleAlert.showError('播放视频时出错：' + error);
    }
  });

  // 卡片点击显示详情
  card.addEventListener('click', () => {
    showVideoDetails(video);
  });

  return card;
}

/**
 * 显示视频详情
 * @param video 视频信息
 */
function showVideoDetails(video: VideoInfo): void {
  const modal = document.createElement('div');
  modal.className = 'modal active';
  
  modal.innerHTML = `
    <div class="modal-content">
      <span class="modal-close">&times;</span>
      <div class="video-details">
        <img src="${video.thumbnail}" alt="${video.title}" style="width: 200px; float: left; margin-right: 20px;">
        <h2>${video.title_cn}</h2>
        <p><strong>时长：</strong>${video.duration}</p>
        <p><strong>分类：</strong>${video.category}</p>
        <p><strong>描述：</strong>${video.description}</p>
        <button class="play-button">播放视频</button>
      </div>
    </div>
  `;

  document.body.appendChild(modal);

  // 播放按钮事件
  const playButton = modal.querySelector('.play-button');
  playButton?.addEventListener('click', async () => {
    try {
      await invoke('play_video', { video: video });
    } catch (error) {
      console.error('Error playing video:', error);
      _simpleAlert.showError('播放视频时出错：' + error);
    }
  });

  // 关闭按钮事件
  const closeBtn = modal.querySelector('.modal-close');
  closeBtn?.addEventListener('click', () => {
    document.body.removeChild(modal);
  });

  // 点击模态框外部关闭
  modal.addEventListener('click', (e) => {
    if (e.target === modal) {
      document.body.removeChild(modal);
    }
  });
}

/**
 * 扫描文件夹并显示视频
 * @returns 
 */
async function selectAndScanFolder(): Promise<void> {
  try {
    let tempVideos: VideoInfo[] = await invoke<VideoInfo[]>('select_and_scan_folder');
    _simpleAlert.showSuccess(`已添加：${tempVideos.length}!`, { duration: 5000 });
    if (tempVideos.length === 0) {
      return;
    }
    _videos = tempVideos;
    displayVideos(tempVideos);
  } catch (error) {
    console.error('Error scanning folder:', error);
    _simpleAlert.showError('扫描文件夹时出错：' + error);
  }
}

/**
 * 显示视频列表
 * @param videos 视频列表
 * @returns 
 */
function displayVideos(videos: VideoInfo[]): void {
  const videoGrid = document.getElementById('video-grid');
  if (!videoGrid) return;

  videoGrid.innerHTML = '';
  if (videos.length === 0) {
    videoGrid.innerHTML = '<div class="no-videos">未找到视频文件</div>';
    return;
  }

  getVideos(_active).forEach(video => {
    videoGrid.appendChild(createVideoCard(video));
  });
}

/**
 * 过滤视频列表
 * @param keyword 过滤视频关键字
 * @returns 
 */
function getVideos(keyword: string): VideoInfo[] {
  let tempVideos:VideoInfo[] = _videos;
  if (keyword === 'tv') {
    _active = 'tv';
    return tempVideos.filter(video => video.is_series);
  }
  if (keyword === 'mv') {
    _active = 'mv';
    return tempVideos.filter(video => !video.is_series);
  }
  if (keyword === 'played') {
    _active = 'played';
    return tempVideos.filter(video => video.play_count > 0).sort((a, b) => b.last_play_time - a.last_play_time);
  }
  _active = 'all';
  return _videos;
}

/**
 * 切换按钮激活状态
 * @param e DOM元素
 */
function toggleButtonActive(e: Element): void {
  document.querySelector("aside.sidebar > nav > ul > li > a.active")?.classList.remove('active');
  e?.classList.add('active');
}

/**
 * 显示设置页面
 */
function showSettings(): void {
  const modal = document.createElement('div');
  modal.className = 'modal active';
  
  modal.innerHTML = `
    <div class="modal-content settings-modal">
      <span class="modal-close">&times;</span>
      <h2>设置</h2>
      <div class="settings-form">
        <h4>默认播放器</h4>
        <div class="form-group">
          <label for="player-path">播放器路径：</label>
          <input type="text" id="player-path" placeholder="例如：C:/Program Files/VLC/vlc.exe">
        </div>
        <div class="form-group">
          <label for="player-type">播放器类型：</label>
          <select id="player-type">
            <option value="vlc">VLC</option>
            <option value="mpv">MPV</option>
            <option value="iina">IINA</option>
            <option value="system">系统默认</option>
          </select>
        </div>
        <div class="form-group">
          <h4>字幕</h4>
          <div class="toggle-settings">
            <label for="auto-subtitle">自动加载字幕<br><span>视频文件同目录的字幕文件夹</span></label>
            <label class="toggle-switch">
              <input type="checkbox" id="auto-subtitle">
              <div class="toggle-switch-background">
                <div class="toggle-switch-handle"></div>
              </div>
            </label>
          </div>
          <div class="toggle-settings">
            <label for="subtitle-language">默认语言：</label>
            <select class="w-auto" id="subtitle-language">
              <option value="chi">chi - 中文 (Chinese)</option>
              <option value="rus">rus - 俄语 (Russian)</option>
              <option value="eng">eng - 英语 (English)</option>
              <option value="fre">fre - 法语 (French)</option>
              <option value="spa">spa - 西班牙语 (Spanish)</option>
              <option value="ger">ger - 德语 (German)</option>
              <option value="ita">ita - 意大利语 (Italian)</option>
              <option value="jpn">jpn - 日语 (Japanese)</option>
              <option value="por">por - 葡萄牙语 (Portuguese)</option>
              <option value="kor">kor - 韩语 (Korean)</option>
            </select>
          </div>
        </div>
        <div class="form-group">
          <h4>TMDB 获取视频信息</h4>
          <div class="toggle-settings">
            <label for="auto-tmdb">自动获取视频信息<br><span>需要TMDB API KEY</span></label>
            <label class="toggle-switch">
              <input type="checkbox" id="auto-tmdb">
              <div class="toggle-switch-background">
                <div class="toggle-switch-handle"></div>
              </div>
            </label>
          </div>
          <div>
            <label for="tmdb-api-key">API KEY：</label>
            <input type="text" id="tmdb-api-key" placeholder="TMDB API KEY">
          </div>
        </div>
        <button class="save-settings">保存设置</button>
      </div>
    </div>
  `;

  document.body.appendChild(modal);

  // 加载已保存的设置
  loadSettings();

  // 保存设置按钮事件
  const saveButton = modal.querySelector('.save-settings');
  saveButton?.addEventListener('click', saveSettings);

  // 关闭按钮事件
  const closeBtn = modal.querySelector('.modal-close');
  closeBtn?.addEventListener('click', () => {
    document.body.removeChild(modal);
  });

  // 点击模态框外部关闭
  modal.addEventListener('click', (e) => {
    if (e.target === modal) {
      document.body.removeChild(modal);
    }
  });
}

/**
 * 保存设置
 */
async function saveSettings(): Promise<void> {
  const playerPath = (document.getElementById('player-path') as HTMLInputElement)?.value;
  const playerType = (document.getElementById('player-type') as HTMLSelectElement)?.value;

  const autoSubtitle = (document.getElementById('auto-subtitle') as HTMLInputElement)?.checked;
  const subtitleLanguage = (document.getElementById('subtitle-language') as HTMLSelectElement)?.value;

  const tmdbApiKey = (document.getElementById('tmdb-api-key') as HTMLInputElement)?.value;
  const auto_tmdb = (document.getElementById('auto-tmdb') as HTMLInputElement)?.checked;

  await invoke('save_settings', { 
    settings: { 
      player_path: playerPath, 
      player_type: playerType,
      auto_subtitle: autoSubtitle,
      subtitle_language: subtitleLanguage,
      tmdb_api_key: tmdbApiKey,
      auto_tmdb: auto_tmdb
    } 
  });

  _simpleAlert.showSuccess('设置已保存!', { duration: 5000 });

  let settingModal = document.querySelector('div.modal.active');
  if (settingModal)
    document.body.removeChild(settingModal);
}

/**
 * 加载设置
 */
async function loadSettings(): Promise<void> {
  const settings = await invoke('load_settings') as Settings;
  if (settings) {
    (document.getElementById('player-path') as HTMLInputElement).value = settings.player_path || '';
    (document.getElementById('player-type') as HTMLSelectElement).value = settings.player_type || 'system';
    (document.getElementById('auto-subtitle') as HTMLInputElement).checked = settings.auto_subtitle || false;
    (document.getElementById('subtitle-language') as HTMLSelectElement).value = settings.subtitle_language || 'chi';
    (document.getElementById('tmdb-api-key') as HTMLInputElement).value = settings.tmdb_api_key || '';
    (document.getElementById('auto-tmdb') as HTMLInputElement).checked = settings.auto_tmdb || false;
  }
}

/**
 * 初始化应用
 */
async function initializeApp(): Promise<void> {
  initializeResizer();
  _simpleAlert = new SimpleAlert();
  
  // 扫描文件夹
  document.querySelector('.scan-button')?.addEventListener('click', (e) => {
    e.preventDefault();
    selectAndScanFolder();
  });

  // 筛选视频-电影
  document.querySelector('.mv-button')?.addEventListener('click', (e) => {
    e.preventDefault();
    toggleButtonActive(e.target as Element);
    displayVideos(getVideos('mv'));
  });

  // 筛选视频-电视剧
  document.querySelector('.tv-button')?.addEventListener('click', (e) => {
    e.preventDefault();
    toggleButtonActive(e.target as Element);
    displayVideos(getVideos('tv'));
  });

  // 筛选视频-全部
  document.querySelector('.all-button')?.addEventListener('click', (e) => {
    e.preventDefault();
    toggleButtonActive(e.target as Element);
    displayVideos(getVideos('all'));
  });

  // 筛选视频-最近播放
  document.querySelector('.recently-played-button')?.addEventListener('click', (e) => {
    e.preventDefault();
    toggleButtonActive(e.target as Element);
    displayVideos(getVideos('played'));
  });

  // 设置按钮
  document.querySelector('.settings-button')?.addEventListener('click', (e) => {
    e.preventDefault();
    toggleButtonActive(e.target as Element);
    showSettings();
  });

  try {
    // 尝试加载缓存的视频
    _videos = await invoke<VideoInfo[]>('get_cached_videos');
    if (_videos.length > 0) {
      displayVideos(_videos);
    } else {
      const videoGrid = document.getElementById('video-grid');
      if (videoGrid) {
        videoGrid.innerHTML = '<div class="no-videos">点击"扫描文件夹"添加视频</div>';
      }
    }
  } catch (error) {
    console.error('Error loading cached videos:', error);
    const videoGrid = document.getElementById('video-grid');
    if (videoGrid) {
      videoGrid.innerHTML = '<div class="no-videos">加载缓存视频失败</div>';
    }
  }

}

window.addEventListener('DOMContentLoaded', initializeApp);
