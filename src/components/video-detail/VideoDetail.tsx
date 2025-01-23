import { VideoInfo } from '../../types';
import './VideoDetail.css';
import { videocommon } from '../../utils';

interface CardProps {
  data: VideoInfo,
}

const VideoDetail = (props: CardProps) => {
  const video = props.data;

  const handlePlayVideo = async () => {
    await videocommon.handlePlayVideo(video);
  }

  return (
    <div className="video-details">
      <img src={videocommon.convertFileSrc(video.thumbnail)} style={{ width: '200px', float: 'left', marginRight: '20px', borderRadius: '5px' }} />
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