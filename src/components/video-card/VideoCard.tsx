import { convertFileSrc } from "@tauri-apps/api/core";
import { VideoInfo } from "../../types.ts";
import "./VideoCard.css";

interface CardProps {
  key: string,
  data: VideoInfo,
  onDelete: (video: VideoInfo) => void;
  onClick: () => void;
}

const VideoCard = (props: CardProps) => {
  const video = props.data;
  const title = video.is_series 
    ? `${video.title} S${video.season.toString().padStart(2, '0')}E${video.episode.toString().padStart(2, '0')}`
    : video.title || video.original_title;

  const handleDeleteClick = async () => {
    props.onDelete(video);
  };

  const isLocalPathExists = (path: string): boolean => {
    if (path.startsWith('/assets')) return false;
    return (/^[a-zA-Z]:\\/.test(path) || /^\//.test(path));
  }

  const load_image = (path: string) => {
    if (isLocalPathExists(path)) {
      return convertFileSrc(path);
    }
    return path;
  }

  return (
    <div className="video-card" onClick={props.onClick}>
      <div className="video-thumbnail">
          <img src={load_image(video.thumbnail)} alt={title} />
          <div className="card-play-button" />
      </div>
      <div className="video-info">
          <h3 className="video-title">
            {title}
          </h3>
          <div className="video-metadata">
            <span className="video-duration">
                {video.duration}
            </span>
            <span className="video-category">
                {video.tags}
            </span>
          </div>
      </div>
      <div className="close-button" title="Close" onClick={(e) => { e.stopPropagation(); handleDeleteClick(); }}>
          ×
      </div>
    </div>
  );
};

export default VideoCard;