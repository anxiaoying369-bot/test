use tauri::State;
use uuid::Uuid;
use crate::models::{RenderConfig};
use crate::state::AppState;
use crate::utils::{get_data_dir};
use crate::ffmpeg;
use std::fs;

#[tauri::command]
pub async fn video_test_ffmpeg() -> Result<String, String> {
    let path = crate::ffmpeg::get_ffmpeg_path();
    let output = crate::utils::tokio_command(&path).arg("-version").output().await.map_err(|e| e.to_string())?;
    if output.status.success() { Ok(String::from_utf8_lossy(&output.stdout).to_string()) } else { Err(String::from_utf8_lossy(&output.stderr).to_string()) }
}

#[tauri::command]
pub async fn video_get_metadata(path: String) -> Result<serde_json::Value, String> {
    let ffprobe = crate::ffmpeg::get_ffprobe_path();
    let output = crate::utils::tokio_command(&ffprobe).args(["-v", "quiet", "-print_format", "json", "-show_format", "-show_streams", &path]).output().await.map_err(|e| e.to_string())?;
    if output.status.success() { let stdout = String::from_utf8_lossy(&output.stdout); serde_json::from_str(&stdout).map_err(|e| e.to_string()) } else { Err(String::from_utf8_lossy(&output.stderr).to_string()) }
}

#[tauri::command]
pub async fn video_run_ffmpeg(app: tauri::AppHandle, task_id: String, args: Vec<String>) -> Result<(), String> {
    crate::ffmpeg::run_ffmpeg_with_progress(task_id, args, app, "processing".to_string()).await
}

#[tauri::command]
pub async fn video_concat_materials(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    material_paths: Vec<String>,
) -> Result<String, String> {
    let task_id = format!("concat_{}", &Uuid::new_v4().to_string()[..8]);
    let output_dir = get_data_dir().join("video_studio").join("output").join(&project_id);
    fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
    let output_path = output_dir.join(format!("{}.mp4", task_id));
    let output_path_str = output_path.to_string_lossy().to_string();

    let list_path = output_dir.join(format!("{}_list.txt", task_id));
    let mut list_content = String::new();
    for p in &material_paths {
        // FFmpeg concat 协议在 Windows 上更喜欢正斜杠，或者需要转义反斜杠
        let safe_p = if cfg!(windows) { p.replace('\\', "/") } else { p.clone() };
        list_content.push_str(&format!("file '{}'\n", safe_p));
    }
    fs::write(&list_path, list_content).map_err(|e| e.to_string())?;

    let args = vec![
        "-y".to_string(), "-f".to_string(), "concat".to_string(), "-safe".to_string(), "0".to_string(),
        "-i".to_string(), list_path.to_string_lossy().to_string(),
        "-c".to_string(), "copy".to_string(),
        output_path_str.clone(),
    ];

    let app_clone = app.clone();
    let task_id_clone = task_id.clone();
    tauri::async_runtime::spawn(async move {
        match crate::ffmpeg::run_ffmpeg_with_progress(task_id_clone.clone(), args, app_clone.clone(), "processing".to_string()).await {
            Ok(_) => {
                let _ = fs::remove_file(list_path);
            }
            Err(e) => {
                eprintln!("Concat failed: {}", e);
            }
        }
    });

    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    db.execute("INSERT INTO video_tasks (id, project_id, type, status, result_path) VALUES (?1, ?2, ?3, ?4, ?5)", (&task_id, &project_id, "concat", "processing", &output_path_str)).map_err(|e| e.to_string())?;

    Ok(task_id)
}

async fn get_video_duration(path: &str) -> Result<f64, String> {
    let ffprobe = ffmpeg::get_ffprobe_path();
    let output = crate::utils::tokio_command(&ffprobe)
        .args([
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            path
        ])
        .output()
        .await
        .map_err(|e| format!("无法执行 ffprobe ({})，请确保已安装 FFmpeg 环境: {}", ffprobe, e))?;

    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    s.parse::<f64>().map_err(|e| format!("Failed to parse duration '{}': {}", s, e))
}

#[tauri::command]
pub async fn video_render_advanced(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    video_paths: Vec<String>,
    bgm_path: Option<String>,
    _config: RenderConfig,
) -> Result<String, String> {
    if video_paths.is_empty() {
        return Err("No video clips provided".to_string());
    }

    let task_id = format!("render_{}", &Uuid::new_v4().to_string()[..8]);
    let project_dir = get_data_dir().join("video_studio").join("materials").join(&project_id);
    fs::create_dir_all(&project_dir).map_err(|e| e.to_string())?;

    let output_path = project_dir.join(format!("output_{}.mp4", task_id));
    let output_path_str = output_path.to_string_lossy().to_string();

    let mut args = vec![];
    for p in &video_paths {
        args.push("-i".to_string());
        args.push(p.clone());
    }

    if let Some(ref bgm) = bgm_path {
        args.push("-i".to_string());
        args.push(bgm.clone());
    }

    let filter = format!("concat=n={}:v=1:a=1", video_paths.len());
    args.push("-filter_complex".to_string());
    args.push(filter);
    args.push("-c:v".to_string());
    args.push("libx264".to_string());
    args.push("-preset".to_string());
    args.push("veryfast".to_string());
    args.push(output_path_str.clone());

    ffmpeg::run_ffmpeg_with_progress(task_id.clone(), args, app, "processing".to_string()).await?;

    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO video_tasks (id, project_id, type, status, result_path) VALUES (?1, ?2, ?3, ?4, ?5)",
        (&task_id, &project_id, "render", "processing", &output_path_str),
    ).map_err(|e| e.to_string())?;

    Ok(task_id)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn video_export_render(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    audio_path: String,
    visual_paths: Vec<String>,
    burn_subtitle: Option<bool>,      // 是否烧录字幕
    subtitle_text: Option<String>,    // 字幕文本（口播文案）
) -> Result<String, String> {
    let all_visual_paths = visual_paths;
    if all_visual_paths.is_empty() {
        return Err("请至少选择一个视觉素材".to_string());
    }

    let audio_duration = get_video_duration(&audio_path).await?;
    let task_id = format!("export_{}", &Uuid::new_v4().to_string()[..8]);
    let output_dir = get_data_dir().join("video_studio").join("output").join(&project_id);
    fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
    let output_path = output_dir.join(format!("{}.mp4", task_id));
    let output_path_str = output_path.to_string_lossy().to_string();

    // 目标分辨率（竖屏 9:16，口播带货默认）
    let (w, h) = (1080, 1920);
    let n = all_visual_paths.len();

    // 判断每个素材是图片还是视频（按扩展名）
    let img_exts = ["png", "jpg", "jpeg", "webp", "bmp", "gif"];
    let is_image: Vec<bool> = all_visual_paths.iter().map(|p| {
        std::path::Path::new(p).extension()
            .and_then(|e| e.to_str())
            .map(|e| img_exts.contains(&e.to_lowercase().as_str()))
            .unwrap_or(false)
    }).collect();

    // 合成规则：
    //  - 图片：每张展示时长随机 0.8~3.0 秒
    //  - 视频：完整播放（用素材自身时长）
    //  - 段间随机转场（xfade，时长不超过相邻段的一半，避免短片段转场出错）
    //  - 素材顺序循环排列，直到铺满音频时长；最后 -shortest 截断到音频时长
    let dur_min = 0.8_f64;
    let dur_max = 3.0_f64;

    // 预获取每个视频素材的实际时长（图片为 None）
    let mut base_video_dur: Vec<f64> = vec![0.0; n];
    for i in 0..n {
        if !is_image[i] {
            base_video_dur[i] = get_video_duration(&all_visual_paths[i]).await.unwrap_or(3.0).max(0.3);
        }
    }

    // 确定性伪随机（不引入 rand 依赖；不同 task_id 组合不同）
    let seed: usize = task_id.bytes().map(|b| b as usize).sum::<usize>().wrapping_add(1);
    let prng = |k: usize| -> f64 {
        let x = seed
            .wrapping_add(k.wrapping_mul(2654435761))
            .wrapping_mul(40503)
            .wrapping_add(12345);
        (x % 100000) as f64 / 100000.0 // 0.0 ~ 1.0
    };

    // 边生成每段时长边累计"有效时长"（含转场重叠），直到铺满音频
    let mut durs: Vec<f64> = Vec::new();
    let mut tds: Vec<f64> = Vec::new(); // tds[k] = 第 k 段与第 k-1 段之间的转场时长（k>=1）
    let mut effective = 0.0_f64;
    let mut k = 0usize;
    while effective < audio_duration + 0.3 && k < 600 {
        let idx = k % n;
        let dur = if is_image[idx] {
            dur_min + prng(k) * (dur_max - dur_min) // 图片随机 0.8~3.0 秒
        } else {
            base_video_dur[idx] // 视频完整时长
        };
        if durs.is_empty() {
            effective = dur;
        } else {
            // 转场时长不超过相邻两段较短者的一半，最低 0.05s，最高 0.5s
            let td = (durs[k - 1].min(dur) * 0.5).clamp(0.05, 0.5);
            tds.push(td);
            effective += dur - td;
        }
        durs.push(dur);
        k += 1;
    }
    let needed = durs.len().max(1);
    // 循环素材索引序列：0,1,...,n-1,0,1,...
    let seq: Vec<usize> = (0..needed).map(|i| i % n).collect();

    // 随机转场池（xfade 支持的过渡类型）
    let transitions = [
        "fade", "wipeleft", "wiperight", "wipeup", "wipedown",
        "slideleft", "slideright", "slideup", "slidedown",
        "circleopen", "circleclose", "dissolve", "pixelize",
        "smoothleft", "smoothright", "fadeblack", "radial",
    ];
    let pick_transition = |k: usize| -> &str {
        transitions[seed.wrapping_add(k.wrapping_mul(2246822519)) % transitions.len()]
    };

    let mut args: Vec<String> = vec!["-y".to_string()];
    // input 0：主音频
    args.push("-i".to_string());
    args.push(audio_path.clone());
    // 每个片段一个独立 input（同一素材可重复出现 = 循环播放）；图片按各自随机时长 loop，视频完整
    for (k, &idx) in seq.iter().enumerate() {
        let p = &all_visual_paths[idx];
        if is_image[idx] {
            args.push("-loop".to_string()); args.push("1".to_string());
            args.push("-t".to_string()); args.push(format!("{:.3}", durs[k]));
        }
        args.push("-i".to_string());
        args.push(p.clone());
    }

    // 滤镜链：每个片段统一 scale+pad 到 WxH、30fps、yuv420p、归零时间戳；
    // 图片裁剪到随机时长；视频不裁剪（完整播放）。
    let mut fc = String::new();
    for (k, &idx) in seq.iter().enumerate() {
        let inp = k + 1; // input 0 是音频，片段从 1 开始
        if is_image[idx] {
            let d = durs[k];
            fc.push_str(&format!(
                "[{inp}:v]scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps=30,trim=duration={d:.3},setpts=PTS-STARTPTS,format=yuv420p[v{k}];"
            ));
        } else {
            // 视频：完整播放，不 trim，仅去音轨/统一规格 + 归零时间戳
            fc.push_str(&format!(
                "[{inp}:v]scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps=30,setpts=PTS-STARTPTS,format=yuv420p[v{k}];"
            ));
        }
    }

    // 段间链式 xfade（随机转场）。单段时无需转场。
    // offset 按每段实际时长累积：running 表示当前累积视频的总时长。
    let final_label = if seq.len() == 1 {
        "v0".to_string()
    } else {
        let mut prev = "v0".to_string();
        let mut running = durs[0];
        for k in 1..seq.len() {
            let td = tds[k - 1]; // 第 k 段与第 k-1 段之间的转场时长
            let offset = (running - td).max(0.0);
            let t = pick_transition(k);
            let out = if k == seq.len() - 1 { "outv".to_string() } else { format!("x{k}") };
            fc.push_str(&format!(
                "[{prev}][v{k}]xfade=transition={t}:duration={td:.3}:offset={offset:.3}[{out}];"
            ));
            running += durs[k] - td;
            prev = out;
        }
        "outv".to_string()
    };
    let mut fc = fc.trim_end_matches(';').to_string();
    let mut map_label = final_label.clone();

    // 可选：烧录字幕。把口播文案按句切分、按字符比例分配到音频时长，生成 SRT，再用 subtitles 滤镜烧进画面。
    if burn_subtitle.unwrap_or(false) {
        if let Some(text) = subtitle_text.as_ref().filter(|t| !t.trim().is_empty()) {
            let srt_path = output_dir.join(format!("{}.srt", task_id));
            build_srt_file(text, audio_duration, &srt_path)?;
            let srt_escaped = escape_subtitle_path(&srt_path.to_string_lossy());
            // 字幕样式：白字黑边、底部居中
            let style = "FontSize=18,PrimaryColour=&H00FFFFFF,OutlineColour=&H00000000,BorderStyle=1,Outline=2,Shadow=0,Alignment=2,MarginV=80";
            fc.push_str(&format!(
                ";[{map_label}]subtitles='{srt_escaped}':force_style='{style}'[vsub]"
            ));
            map_label = "vsub".to_string();
        }
    }

    args.push("-filter_complex".to_string());
    args.push(fc);
    args.push("-map".to_string()); args.push(format!("[{map_label}]"));
    args.push("-map".to_string()); args.push("0:a".to_string());
    args.push("-c:v".to_string()); args.push("libx264".to_string());
    args.push("-preset".to_string()); args.push("veryfast".to_string());
    args.push("-crf".to_string()); args.push("23".to_string());
    args.push("-pix_fmt".to_string()); args.push("yuv420p".to_string());
    args.push("-c:a".to_string()); args.push("aac".to_string());
    args.push("-b:a".to_string()); args.push("192k".to_string());
    // 以音频时长为准截断（视觉总长 = n*seg ≥ 音频时长）
    args.push("-shortest".to_string());
    args.push(output_path_str.clone());

    ffmpeg::run_ffmpeg_with_progress(task_id.clone(), args, app, "processing".to_string()).await?;

    let db = state.video_db.lock().map_err(|e| e.to_string())?;
    // run_ffmpeg_with_progress 已 await 完成，这里状态记为 completed
    db.execute(
        "INSERT INTO video_tasks (id, project_id, type, status, result_path) VALUES (?1, ?2, ?3, ?4, ?5)",
        (&task_id, &project_id, "export", "completed", &output_path_str),
    ).map_err(|e| e.to_string())?;
    // 成片作为视频素材入库，方便在「素材库」直接预览
    let mat_id = Uuid::new_v4().to_string();
    db.execute(
        "INSERT INTO video_materials (id, project_id, type, local_path, source) VALUES (?1, ?2, ?3, ?4, ?5)",
        (&mat_id, &project_id, "video", &output_path_str, "export"),
    ).map_err(|e| e.to_string())?;

    // 返回成片绝对路径，前端可直接预览
    Ok(output_path_str)
}

/// 把口播文案按句切分、按字符数比例分配到 total 秒，生成 SRT 字幕文件。
fn build_srt_file(text: &str, total: f64, path: &std::path::Path) -> Result<(), String> {
    let seps = ['。', '！', '？', '；', '.', '!', '?', ';', '\n', '，', ','];
    let mut sentences: Vec<String> = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        if seps.contains(&ch) {
            let t = cur.trim().to_string();
            if !t.is_empty() { sentences.push(t); }
            cur.clear();
        } else {
            cur.push(ch);
        }
    }
    if !cur.trim().is_empty() { sentences.push(cur.trim().to_string()); }
    if sentences.is_empty() { return Err("字幕文本为空".to_string()); }

    let total_chars: usize = sentences.iter().map(|s| s.chars().count()).sum::<usize>().max(1);
    let mut srt = String::new();
    let mut t = 0.0_f64;
    for (i, s) in sentences.iter().enumerate() {
        let dur = total * (s.chars().count() as f64 / total_chars as f64);
        let start = t;
        let end = (t + dur).min(total);
        srt.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1, fmt_srt_ts(start), fmt_srt_ts(end), s
        ));
        t = end;
    }
    std::fs::write(path, srt).map_err(|e| format!("写入字幕文件失败: {}", e))?;
    Ok(())
}

fn fmt_srt_ts(s: f64) -> String {
    let s = s.max(0.0);
    let h = (s / 3600.0).floor() as u64;
    let m = ((s % 3600.0) / 60.0).floor() as u64;
    let sec = (s % 60.0).floor() as u64;
    let ms = ((s - s.floor()) * 1000.0).round() as u64;
    format!("{:02}:{:02}:{:02},{:03}", h, m, sec, ms)
}

/// 转义 subtitles 滤镜里的路径：单引号包裹下，需转义 \ : ' （Windows 盘符 C: 也要处理）
fn escape_subtitle_path(p: &str) -> String {
    p.replace('\\', "/")        // Windows 反斜杠 → 正斜杠
     .replace(':', "\\:")       // 冒号转义（Windows 盘符）
     .replace('\'', "\\'")      // 单引号转义
}
