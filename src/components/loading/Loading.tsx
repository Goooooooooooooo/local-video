import { useEffect, useState } from "react";
import "./Loading.css"

interface LoadingProps {
  duration?: number; // 以毫秒为单位的关闭时间
}

const Loading: React.FC<LoadingProps> = ({ duration }) => {
  const [visible, setVisible] = useState(true);

  useEffect(() => {
    if (duration) {
      const timer = setTimeout(() => {
        setVisible(false);
      }, duration);
      return () => clearTimeout(timer);
    }
  }, [duration]);

  if (!visible) return null;

  return (
    <div className="loading-overlay">
      <div className="loading-display">
        <svg className="ld" width="240" height="240" viewBox="0 0 240 240">
        <circle className="ld__ring ld__ring--a" cx="120" cy="120" r="105" fill="none" stroke="#000" stroke-width="20" stroke-dasharray="0 660" stroke-dashoffset="-330" stroke-linecap="round"></circle>
        <circle className="ld__ring ld__ring--b" cx="120" cy="120" r="35" fill="none" stroke="#000" stroke-width="20" stroke-dasharray="0 220" stroke-dashoffset="-110" stroke-linecap="round"></circle>
        <circle className="ld__ring ld__ring--c" cx="85" cy="120" r="70" fill="none" stroke="#000" stroke-width="20" stroke-dasharray="0 440" stroke-linecap="round"></circle>
        <circle className="ld__ring ld__ring--d" cx="155" cy="120" r="70" fill="none" stroke="#000" stroke-width="20" stroke-dasharray="0 440" stroke-linecap="round"></circle>
        </svg>
      </div>
    </div>
  )
}

export default Loading;