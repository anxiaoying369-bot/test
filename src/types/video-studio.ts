export interface VideoProject {
  id: string;
  title: string;
  description?: string;
  config?: any;
  status: string;
  is_locked?: boolean;
  locked_at?: string;
  final_video_path?: string;
  created_at?: string;
  updated_at?: string;
}

export interface VideoMaterial {
  id: string;
  project_id: string;
  material_type: string;
  source?: string;
  local_path?: string;
  remote_url?: string;
  meta?: any;
  created_at?: string;
}

export interface VideoTask {
  id: string;
  project_id?: string;
  task_type: string;
  status: string;
  progress: number;
  result_path?: string;
  error_msg?: string;
  created_at?: string;
  updated_at?: string;
}

export interface FfmpegProgress {
  task_id: string;
  percentage: number;
  speed: string;
  time: string;
  stage: string;
}
