import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import simpleAlert from "./components/simplealert";
import { VideoInfo } from "./types";

export const videocommon = {
  /**
   * 判断是否是网络路径
   * @param path 文件路径
   * @returns boolean
   */
  isLocalPathExists: (path: string): boolean => {
    if (path.startsWith('/assets')) return false;
    return (/^[a-zA-Z]:\\/.test(path) || /^\//.test(path));
  },
  /**
   * 转成tauri前端协议可显示的路径
   * @param path 文件路径
   * @returns string
   */
  convertFileSrc: (path: string): string => {
    if (videocommon.isLocalPathExists(path)) {
      return convertFileSrc(path);
    }
    return path;
  },
  /**
   * 播放视频，调用Rust方法
   * @param video VideoInfo
   */
  handlePlayVideo: async (video: VideoInfo) => {
    try {
      await invoke('play_video', { video: video });
    } catch (error) {
      console.error('Error playing video:', error);
      simpleAlert.error('播放视频时出错：' + error);
    }
  },
  /**
   * 延时
   * @param duration 毫秒，默认1000
   */
  sleep: async(duration: number = 1000) => {
    await new Promise((resolve) => setTimeout(resolve, duration));
  }
};