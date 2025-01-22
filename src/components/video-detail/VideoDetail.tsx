import { invoke } from '@tauri-apps/api/core';
import { VideoInfo } from '../../types';
import './VideoDetail.css';
import simpleAlert from '../simplealert';

interface CardProps {
  data: VideoInfo,
}

const VideoDetail = (props: CardProps) => {
  const video = props.data;

  const handlePlayVideo = async () => {
    try {
      await invoke('play_video', { video: video });
    } catch (error) {
      console.error('Error playing video:', error);
      simpleAlert.error('播放视频时出错：' + error);
    }
  }

  return (
    <div className="video-details">
      <img src={video.thumbnail} style={{ width: '200px', float: 'left', marginRight: '20px' }} />
      <h2>{video.is_series ? video.episode_title : video.title}</h2>
      <p>
        <strong>时长：</strong>{video.duration}
      </p>
      <p>
        <strong>分类：</strong>{video.category}
      </p>
      <p>
        <strong>标签：</strong>{video.tags}
      </p>
      <p>
        <strong>描述：</strong>{video.is_series ? video.episode_overview : video.description}
      </p>
      <button className="play-button" onClick={handlePlayVideo}>播放视频</button>
    </div>
  );
};

export default VideoDetail;