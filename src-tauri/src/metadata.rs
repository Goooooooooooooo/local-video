use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

/// 定义 EBML 头部和一些常用的元素 ID 常量。
const EBML_HEADER_ID: u32 = 0x1A45DFA3;
const SEGMENT_ID: u32 = 0x18538067;
const INFO_ID: u32 = 0x1549A966;
const DURATION_ID: u32 = 0x4489;
const TIMECODE_SCALE_ID: u32 = 0x2AD7B1;

/// 读取 EBML 中的 VINT（可变长度整数）。
fn read_vint<R: Read>(reader: &mut R) -> Result<u64, String> {
    let mut first_byte = [0u8; 1];
    reader.read_exact(&mut first_byte).map_err(|e| e.to_string())?;

    let mut length = 0;
    let mut value = 0u64;

    for i in 0..8 {
        if first_byte[0] & (0x80 >> i) != 0 {
            length = i + 1;
            value = (first_byte[0] & !(0xFF << (8 - length))) as u64;
            break;
        }
    }

    if length == 0 {
        return Err("Invalid VINT encoding".into());
    }

    let mut buffer = [0u8; 8];
    reader.read_exact(&mut buffer[..length - 1]).map_err(|e| e.to_string())?;
    for &b in &buffer[..length - 1] {
        value = (value << 8) | b as u64;
    }

    Ok(value)
}

/// 读取 EBML 元素的 ID。
fn read_element_id<R: Read>(reader: &mut R) -> Result<u32, String> {
    let mut id = [0u8; 1];
    reader.read_exact(&mut id).map_err(|e| e.to_string())?;
    let mut element_id = id[0] as u32;

    let bytes_to_read = match element_id {
        _ if element_id & 0x80 == 0x80 => 0,
        _ if element_id & 0x40 == 0x40 => 1,
        _ if element_id & 0x20 == 0x20 => 2,
        _ if element_id & 0x10 == 0x10 => 3,
        _ => return Err("Invalid element ID".into()),
    };

    for _ in 0..bytes_to_read {
        let mut byte = [0u8; 1];
        reader.read_exact(&mut byte).map_err(|e| e.to_string())?;
        element_id = (element_id << 8) | byte[0] as u32;
    }

    Ok(element_id)
}

/// 将大端字节序的向量转换为 u64。
fn bytes_to_u64(buffer: &[u8]) -> u64 {
    buffer.iter().fold(0u64, |acc, &b| (acc << 8) | b as u64)
}

/// 将大端字节序的向量转换为 f64。
fn bytes_to_f64(buffer: &[u8]) -> f64 {
    match buffer.len() {
        4 => f32::from_bits(u32::from_be_bytes(buffer.try_into().unwrap())) as f64,
        8 => f64::from_be_bytes(buffer.try_into().unwrap()),
        _ => 0.0,
    }
}

/// 定义用于存储元数据信息的结构体
#[derive(Debug)]
#[allow(dead_code)]
pub struct MkvMetadata {
    pub timecode_scale: u64,
    pub duration: f64,
    pub video_duration_seconds: f64,
}

/// 提取 MKV 文件的元数据信息。
fn get_mkv_metadata(file_path: &str) -> Result<MkvMetadata, String> {
    let file = File::open(file_path).map_err(|e| e.to_string())?;
    let mut reader = BufReader::with_capacity(512 * 1024, file); // 增大缓冲区以提高性能。

    let mut header = [0u8; 4];
    reader.read_exact(&mut header).map_err(|e| e.to_string())?;
    if bytes_to_u64(&header) != EBML_HEADER_ID as u64 {
        return Err("Invalid MKV file".into());
    }

    let ebml_header_size = read_vint(&mut reader)?;
    reader.seek(SeekFrom::Current(ebml_header_size as i64)).map_err(|e| e.to_string())?;

    let segment_id = read_element_id(&mut reader).map_err(|e| e.to_string())?;
    if segment_id != SEGMENT_ID {
        return Err("Invalid Segment element".into());
    }

    let segment_size = read_vint(&mut reader)?;
    let segment_end = reader.seek(SeekFrom::Current(0)).map_err(|e| e.to_string())? + segment_size;

    let mut timecode_scale: Option<u64> = None;
    let mut duration: Option<f64> = None;

    while reader.seek(SeekFrom::Current(0)).map_err(|e| e.to_string())? < segment_end {
        let element_id = read_element_id(&mut reader)?;
        let element_size = read_vint(&mut reader)?;

        if element_id == INFO_ID {
            let info_end = reader.seek(SeekFrom::Current(0)).map_err(|e| e.to_string())? + element_size;
            while reader.seek(SeekFrom::Current(0)).map_err(|e| e.to_string())? < info_end {
                let info_element_id = read_element_id(&mut reader)?;
                let info_element_size = read_vint(&mut reader)?;

                match info_element_id {
                    TIMECODE_SCALE_ID => {
                        let mut buffer = [0u8; 8]; // 限制最大读取长度。
                        reader.read_exact(&mut buffer[..info_element_size as usize]).map_err(|e| e.to_string())?;
                        timecode_scale = Some(bytes_to_u64(&buffer[..info_element_size as usize]));
                    }
                    DURATION_ID => {
                        let mut buffer = [0u8; 8]; // 限制最大读取长度。
                        reader.read_exact(&mut buffer[..info_element_size as usize]).map_err(|e| e.to_string())?;
                        duration = Some(bytes_to_f64(&buffer[..info_element_size as usize]));
                    }
                    _ => {
                        // 打印未处理的元素信息
                        println!("Unknown element ID: {:#X}, size: {}", info_element_id, info_element_size);
                        reader.seek(SeekFrom::Current(info_element_size as i64)).map_err(|e| e.to_string())?;
                    }
                }

                if timecode_scale.is_some() && duration.is_some() {
                    break;
                }
            }
        } else {
            reader.seek(SeekFrom::Current(element_size as i64)).map_err(|e| e.to_string())?;
        }

        if timecode_scale.is_some() && duration.is_some() {
            break;
        }
    }

    let timecode_scale = timecode_scale.ok_or("Missing TimecodeScale in MKV metadata")?;
    let duration = duration.ok_or("Missing Duration in MKV metadata")?;

    let video_duration_seconds = (duration * timecode_scale as f64) / 1_000_000_000.0;

    Ok(MkvMetadata {
        timecode_scale,
        duration,
        video_duration_seconds,
    })
}

/// 公共接口，用于调用元数据解析。
pub fn mkv_metadata(file_path: &str) -> Result<MkvMetadata, String> {
    get_mkv_metadata(file_path)
}


#[cfg(test)]
mod tests {
    use std::io;

    use super::*;

    #[test]
    fn test_read_vint() {
        let data = vec![0x81];
        let mut cursor = io::Cursor::new(data);
        let result = read_vint(&mut cursor);
        assert_eq!(result.unwrap(), 1);

        let data = vec![0x40, 0x01];
        let mut cursor = io::Cursor::new(data);
        let result = read_vint(&mut cursor);
        assert_eq!(result.unwrap(), 1);

        let data = vec![0x20, 0x00, 0x01];
        let mut cursor = io::Cursor::new(data);
        let result = read_vint(&mut cursor);
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_read_element_id() {
        let data = vec![0x1A];
        let mut cursor = io::Cursor::new(data);
        let result = read_element_id(&mut cursor);
        assert_eq!(result.unwrap(), 0x1A);

        let data = vec![0x40, 0x00];
        let mut cursor = io::Cursor::new(data);
        let result = read_element_id(&mut cursor);
        assert_eq!(result.unwrap(), 0x4000);

        let data = vec![0x20, 0x00, 0x00];
        let mut cursor = io::Cursor::new(data);
        let result = read_element_id(&mut cursor);
        assert_eq!(result.unwrap(), 0x200000);
    }

    #[test]
    fn test_get_video_metadata() {
        // 创建一个临时文件，写入测试数据
        let temp_file_path = "C:\\Users\\yzok0\\Videos\\Transformers.One.2024.HDR.2160p.WEB.h265-ETHEL[TGx]\\Transformers.One.2024.HDR.2160p.WEB.h265-ETHEL.mkv";
        // let mut temp_file = File::create(temp_file_path).unwrap();
        // temp_file.write_all(b"\x1A\x45\xDF\xA3").unwrap(); // 文件头
        // temp_file.write_all(&[0x81, 0x00]).unwrap(); // EBML头部
        // temp_file.write_all(&[0x18, 0x53, 0x80, 0x67]).unwrap(); // Segment元素
        // temp_file.write_all(&[0x81, 0x00]).unwrap(); // Segment大小
        // temp_file.write_all(&[0x15, 0x49, 0xA9, 0x66]).unwrap(); // Info元素
        // temp_file.write_all(&[0x81, 0x00]).unwrap(); // Info大小
        // temp_file.write_all(&[0x2A, 0xD7, 0xB1]).unwrap(); // TimecodeScale元素
        // temp_file.write_all(&[0x81, 0x01]).unwrap(); // TimecodeScale大小和值
        // temp_file.write_all(&[0x44, 0x89]).unwrap(); // Duration元素
        // temp_file.write_all(&[0x84, 0x00, 0x00, 0x00, 0x01]).unwrap(); // Duration大小和值

        // 测试 get_video_metadata 函数
        let result = mkv_metadata(temp_file_path);
        println!("{:?}", result);
        assert!(result.is_ok());

        let episode_pattern = regex::Regex::new(r"S\d{2}E\d{2}").ok(); // 匹配剧集编号 SxxExx
        // let language_keywords = ["zh", "chs", "cn", "cht", "chinese", "chr", "简体", "简中", "繁中"];
        // let language_pattern = regex::Regex::new(&format!(r"({})", language_keywords.join("|"))).ok(); // 匹配语言关键字    

        if let Some(ep_pattern) = &episode_pattern {
            println!("Episode: {}", ep_pattern.is_match(&temp_file_path));
        }
        // 删除临时文件
        // std::fs::remove_file(temp_file_path).unwrap();
    }
}