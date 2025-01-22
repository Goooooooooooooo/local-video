import { useState, useEffect, memo, useCallback } from 'react';
import './Settings.css';
import { Settings } from '../types';
import { invoke } from '@tauri-apps/api/core';
import simpleAlert from '../components/simplealert';

const LANGUAGE_OPTIONS = [
  { value: 'chi', label: 'chi - 中文 (Chinese)' },
  { value: 'rus', label: 'rus - 俄语 (Russian)' },
  { value: 'eng', label: 'eng - 英语 (English)' },
  { value: 'fre', label: 'fre - 法语 (French)' },
  { value: 'spa', label: 'spa - 西班牙语 (Spanish)' },
  { value: 'ger', label: 'ger - 德语 (German)' },
  { value: 'ita', label: 'ita - 意大利语 (Italian)' },
  { value: 'jpn', label: 'jpn - 日语 (Japanese)' },
  { value: 'por', label: 'por - 葡萄牙语 (Portuguese)' },
  { value: 'kor', label: 'kor - 韩语 (Korean)' }
];

const PLAYER_OPTIONS = [
  { value: 'vlc', label: 'VLC' },
  { value: 'mpv', label: 'MPV' },
  { value: 'iina', label: 'IINA' },
  { value: 'system', label: '系统默认' }
];

const ToggleSwitch = memo(({ id, checked, onChange, label, description }: {
  id: string;
  checked: boolean;
  onChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
  label: string;
  description: string;
}) => (
  <div className="toggle-settings">
    <label htmlFor={id}>
      {label}
      <br />
      <span>{description}</span>
    </label>
    <label className="toggle-switch">
      <input type="checkbox" id={id} checked={checked} onChange={onChange} />
      <div className="toggle-switch-background">
        <div className="toggle-switch-handle" />
      </div>
    </label>
  </div>
));

const SettingsPage = () => {
  const [settings, setSettings] = useState<Settings | null>(null);

  useEffect(() => {
    // 加载已保存的设置
    const loadSettings = async () => {
      const settings = await invoke('load_settings') as Settings;
      setSettings(settings);
    };
    loadSettings();
  }, []);

  const handleInputChange = useCallback((e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    const { id, value } = e.target;
    setSettings((prev) => prev ? { ...prev, [id]: value } : null);
  }, []);

  const handleToggleChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const { id, checked } = e.target;
    setSettings((prev) => prev ? { ...prev, [id]: checked } : null);
  }, []);

  const handleSave = async () => {
    if (settings) {
      // console.log('Settings saved:', settings);
      await invoke('save_settings', { settings: settings });
      simpleAlert.success('设置已保存!', { duration: 5000 });
    }
  };

  if (!settings) {
    return <div>加载中...</div>;
  }

  return (
    <div>
      <h2>设置</h2>
      <div className="settings-form">
        <h4>默认播放器</h4>
        <div className="form-group">
          <label htmlFor="player-path">播放器路径：</label>
          <input
            type="text"
            id="player_path"
            placeholder="例如：C:/Program Files/VLC/vlc.exe"
            value={settings.player_path}
            onChange={handleInputChange}
          />
        </div>
        <div className="form-group">
          <label htmlFor="player-type">播放器类型：</label>
          <select id="player_type" value={settings.player_type} onChange={handleInputChange}>
            {PLAYER_OPTIONS.map(option => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </div>
        <div className="form-group">
          <h4>字幕</h4>
          <ToggleSwitch
            id="auto_subtitle"
            checked={settings.auto_subtitle}
            onChange={handleToggleChange}
            label="自动加载字幕"
            description="视频文件同目录的字幕文件夹"
          />
          <div className="toggle-settings">
            <label htmlFor="subtitle-language">默认语言：</label>
            <select
              className="w-auto"
              id="subtitle_language"
              value={settings.subtitle_language}
              onChange={handleInputChange}
            >
              {LANGUAGE_OPTIONS.map(option => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </div>
        </div>
        <div className="form-group">
          <h4>TMDB 获取视频信息<br/><span>需要TMDB API KEY</span></h4>
          <ToggleSwitch
            id="auto_tmdb"
            checked={settings.auto_tmdb}
            onChange={handleToggleChange}
            label="自动获取视频信息"
            description=""
          />
          <ToggleSwitch
            id="auto_tmdb_poster"
            checked={settings.auto_tmdb_poster}
            onChange={handleToggleChange}
            label="自动下载海报"
            description=""
          />
          <div>
            <label htmlFor="tmdb-api-key">API KEY：</label>
            <input
              type="text"
              id="tmdb_api_key"
              placeholder="TMDB API KEY"
              value={settings.tmdb_api_key}
              onChange={handleInputChange}
            />
          </div>
        </div>
        <button className="save-settings" onClick={handleSave}>保存设置</button>
      </div>
    </div>
  );
};

export default SettingsPage;