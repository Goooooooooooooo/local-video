import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import VideoCard from "./components/video-card/VideoCard";
import { VideoInfo } from "./types";
import simpleAlert from "./components/simplealert";
import Modal from "./components/modal/Modal";
import VideoDetail from "./components/video-detail/VideoDetail";
import SettingsPage from "./pages/Settings";

function App() {
  const [leftWidth, setLeftWidth] = useState(200); // 左侧初始宽度
  const [isResizing, setIsResizing] = useState(false); // 是否正在拖动
  const [videos, setVideos] = useState<VideoInfo[]>([]);
  const [filter, setFilter] = useState<string>("all"); // 当前过滤条件
  const [selectedVideo, setSelectedVideo] = useState<VideoInfo | null>(null);
  const [currentPage, setCurrentPage] = useState<string>("home"); // 当前页面

  // 开始拖动
  const handleMouseDown = () => {
    setIsResizing(true);
  };

  // 拖动中
  const handleMouseMove = (e: { clientX: any; }) => {
    if (isResizing) {
      const newWidth = e.clientX; // 鼠标的 X 坐标即为左侧宽度
      if (newWidth > 100 && newWidth < window.innerWidth - 100) {
        setLeftWidth(newWidth);
      }
    }
  };

  // 结束拖动
  const handleMouseUp = () => {
    setIsResizing(false);
  };

  useEffect(() => {
    // 绑定全局事件
    if (isResizing) {
      window.addEventListener("mousemove", handleMouseMove);
      window.addEventListener("mouseup", handleMouseUp);
    } else {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    }
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isResizing]);

  // 双击重置宽度
  const handleDoubleClick = () => {
    setLeftWidth(200); // 恢复默认宽度
  };
  
  useEffect(() => {
    const fetchCachedVideos = async () => {
      try {
        // 尝试加载缓存的视频
        const cachedVideos = await invoke<VideoInfo[]>('get_cached_videos');
        setVideos(cachedVideos);
      } catch (error) {
        console.error('Error loading cached videos:', error);
        simpleAlert.error(error as string);
        // const videoGrid = document.getElementById('video-grid');
        // if (videoGrid) {
        //   videoGrid.innerHTML = '<div class="no-videos">加载缓存视频失败</div>';
        // }
      }
    };
    fetchCachedVideos();
  }, []);

  /**
   * 过滤视频列表
   * @param keyword 过滤视频关键字
   * @returns 
   */
  const getVideos = (keyword: string): VideoInfo[] => {
    let tempVideos:VideoInfo[] = videos;
    if (keyword === 'tv') {
      return tempVideos.filter(video => video.is_series);
    }
    if (keyword === 'mv') {
      return tempVideos.filter(video => !video.is_series);
    }
    if (keyword === 'played') {
      return tempVideos.filter(video => video.play_count > 0).sort((a, b) => b.last_play_time - a.last_play_time);
    }
    return videos;
  }

  const handleScanFoldersClick = async () => {
    console.log('扫描文件夹');
    try {
      let tempVideos: VideoInfo[] = await invoke<VideoInfo[]>('select_and_scan_folder');
      if (tempVideos.length === 0) {
        return;
      }
      simpleAlert.success(`已添加：${tempVideos.length}!`, { duration: 5000 });
      setVideos([...videos, ...tempVideos]);
    } catch (error) {
      console.error('Error scanning folder:', error);
      simpleAlert.error('扫描文件夹时出错：' + error);
    }
  };

  const handleSettingsClick = () => {
    console.log('设置');
    setFilter('');
    setCurrentPage('Settings');
  };

  const handleDeleteVideo = async (video: VideoInfo) => {
    try {
      simpleAlert.confirm('您确定要删除这个项目吗？', 
        `<div style="font-size:16px;"><input type="checkbox"> 同时删除文件</div>`, 
        async (confirmed, isChecked) => {
            if (confirmed) {
                if (isChecked) {
                    console.log('用户确认删除，同时删除文件', video.path);
                    await invoke('remove_video', { id: video.id });
                    await invoke('delete_folder_if_exists', { filePath: video.path });
                } else {
                    console.log('用户确认删除，不删除文件');
                    await invoke('remove_video', { id: video.id });
                }
                setVideos(videos.filter(item => item.id !== video.id));
            } else {
                console.log('用户取消删除');
            }
        }
      );
    } catch (error) {
      console.error('Error delete video:', error);
      simpleAlert.error('删除视频出错：' + error);
    }
  };

  const handleCurrentPage = (keyword: string) => {
    setFilter(keyword);
    if (currentPage !== 'home') {
      setCurrentPage('home');
    }
  };

  const handleCardClick = (video: VideoInfo) => {
    // console.log("🚀 ~ handleCardClick ~ video:", video)
    setSelectedVideo(video);
  };

  const handleCloseModal = () => {
    setSelectedVideo(null);
  };

  return (
    <main className="app-container">
        <aside className="sidebar" style={{ width: `${leftWidth}px` }}>
          <nav>
            <ul>
              <li><a href="#" className={`all-button ${filter === 'all' ? 'active' : null}`} onClick={() => handleCurrentPage("all")}>所有视频</a></li>
              <li><a href="#" className={`recently-played-button ${filter === 'played' ? 'active' : null}`} onClick={() => handleCurrentPage("played")}>最近播放</a></li>
              <li><a href="#" className={`mv-button ${filter === 'mv' ? 'active' : null}`} onClick={() => handleCurrentPage("mv")}>电影</a></li>
              <li><a href="#" className={`tv-button ${filter === 'tv' ? 'active' : null}`} onClick={() => handleCurrentPage("tv")}>剧集</a></li>
              <li><a href="#" className="scan-button" onClick={handleScanFoldersClick}>扫描文件夹</a></li>
              <li><a href="#" className={`settings-button ${currentPage === 'Settings' ? 'active' : null}`} onClick={handleSettingsClick}>设置</a></li>
            </ul>
          </nav>
        </aside>
        <div className="resizer" id="resizer"
          onDoubleClick={handleDoubleClick}
          onMouseDown={handleMouseDown} style={{ cursor: "ew-resize", left: `${leftWidth}px` }}></div>
        <div className="content">
        {
        currentPage === "home" && 
          <div className="video-grid" id="video-grid">
            {getVideos(filter).map((video) => (
                <VideoCard key={video.id} data={video}  onDelete={handleDeleteVideo} onClick={() => handleCardClick(video)} />
              ))}
            <Modal isOpen={selectedVideo !== null} onClose={handleCloseModal}>
              {selectedVideo && (<VideoDetail data={selectedVideo} />)}
            </Modal>
          </div>
        }
        {
          currentPage === "Settings" && <SettingsPage />
        }
        </div>
    </main>
  );
}

export default App;
