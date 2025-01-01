import { invoke } from "@tauri-apps/api/core";

// 手动定义接口
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

interface Settings {
    player_path: string;
    player_type: string;
}

// 初始化调整大小功能
function initializeResizer() {
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

// 创建视频卡片
function createVideoCard(video: VideoInfo): HTMLElement {
  const card = document.createElement('div');
  card.className = 'video-card';
  
  const title = video.is_series 
        ? `${video.series_title} S${video.season.toString().padStart(2, '0')}E${video.episode.toString().padStart(2, '0')}`
        : video.title_cn || video.title;

  card.innerHTML = `
    <div class="video-thumbnail">
      <img src="${video.thumbnail}" alt="${title}">
      <div class="card-play-button"></div>
    </div>
    <div class="video-info">
      <h3 class="video-title">${title}</h3>
      <div class="video-metadata">
        <span class="video-duration">${video.duration}</span>
        <span class="video-category">${video.category}</span>
      </div>
    </div>
  `;

  // 播放按钮点击事件
  const cardPlayButton = card.querySelector('.card-play-button');
  cardPlayButton?.addEventListener('click', async (e) => {
    e.stopPropagation(); // 阻止事件冒泡
    try {
      await invoke('play_video', { path: video.path });
    } catch (error) {
      console.error('Error playing video:', error);
      alert('播放视频时出错：' + error);
    }
  });

  // 卡片点击显示详情
  card.addEventListener('click', () => {
    showVideoDetails(video);
  });

  return card;
}

function showVideoDetails(video: VideoInfo) {
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
      await invoke('play_video', { path: video.path });
    } catch (error) {
      console.error('Error playing video:', error);
      alert('播放视频时出错：' + error);
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

async function selectAndScanFolder() {
  try {
    const videos = await invoke<VideoInfo[]>('select_and_scan_folder');
    console.log(videos);
    displayVideos(videos);
  } catch (error) {
    console.error('Error scanning folder:', error);
    alert('扫描文件夹时出错：' + error);
  }
}

function displayVideos(videos: VideoInfo[]) {
  const videoGrid = document.getElementById('video-grid');
  if (!videoGrid) return;

  videoGrid.innerHTML = '';
  if (videos.length === 0) {
    videoGrid.innerHTML = '<div class="no-videos">未找到视频文件</div>';
    return;
  }

  videos.forEach(video => {
    videoGrid.appendChild(createVideoCard(video));
  });
}

// 添加设置按钮到菜单栏
function addSettingsButton() {
  const nav = document.querySelector('.sidebar nav ul');
  if (nav) {
    const settingsButton = document.createElement('li');
    settingsButton.innerHTML = '<a href="#" class="settings-button">设置</a>';
    settingsButton.querySelector('a')?.addEventListener('click', (e) => {
      e.preventDefault();
      showSettings();
    });
    nav.appendChild(settingsButton);
  }
}

// 显示设置页面
function showSettings() {
  const modal = document.createElement('div');
  modal.className = 'modal active';
  
  modal.innerHTML = `
    <div class="modal-content settings-modal">
      <span class="modal-close">&times;</span>
      <h2>播放器设置</h2>
      <div class="settings-form">
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

// 保存设置
async function saveSettings() {
  const playerPath = (document.getElementById('player-path') as HTMLInputElement)?.value;
  const playerType = (document.getElementById('player-type') as HTMLSelectElement)?.value;

  await invoke('save_settings', { 
    settings: { 
      player_path: playerPath, 
      player_type: playerType 
    } 
  });

  alert('设置已保存');
}

// 加载设置
async function loadSettings() {
  const settings = await invoke('load_settings') as Settings;
  if (settings) {
    (document.getElementById('player-path') as HTMLInputElement).value = settings.player_path || '';
    (document.getElementById('player-type') as HTMLSelectElement).value = settings.player_type || 'system';
  }
}

// 初始化应用
async function initializeApp() {
  initializeResizer();

  // 添加扫描按钮到侧边栏
  const nav = document.querySelector('.sidebar nav ul');
  if (nav) {
    const scanButton = document.createElement('li');
    scanButton.innerHTML = '<a href="#" class="scan-button">扫描文件夹</a>';
    scanButton.querySelector('a')?.addEventListener('click', (e) => {
      e.preventDefault();
      selectAndScanFolder();
    });
    nav.appendChild(scanButton);
  }

  try {
    // 尝试加载缓存的视频
    const videos = await invoke<VideoInfo[]>('get_cached_videos');
    if (videos.length > 0) {
      displayVideos(videos);
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

  // 初始化时添加设置按钮
  addSettingsButton();
}

window.addEventListener('DOMContentLoaded', initializeApp);
