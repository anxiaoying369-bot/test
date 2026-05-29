import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { VideoProject, VideoMaterial } from '../types/video-studio';

export function useVideoProjects() {
  const projects = ref<VideoProject[]>([]);
  const currentProject = ref<VideoProject | null>(null);

  const loadProjects = async () => {
    try {
      projects.value = await invoke<VideoProject[]>('video_list_projects');
    } catch (e) {
      console.error('Failed to load projects:', e);
    }
  };

  const createProject = async () => {
    const title = prompt('请输入项目名称', `新项目 ${new Date().toLocaleString()}`);
    if (!title) return;
    const newId = crypto.randomUUID();
    const project: VideoProject = {
      id: newId,
      title,
      status: 'draft',
    };
    try {
      await invoke('video_upsert_project', { project });
      await loadProjects();
      currentProject.value = projects.value.find(p => p.id === newId) || null;
    } catch (e) {
      alert('创建项目失败: ' + e);
    }
  };

  const selectProject = (p: VideoProject) => {
    currentProject.value = p;
  };

  const deleteProject = async (id: string) => {
    if (!confirm('确定要删除该项目及其所有素材吗？')) return;
    try {
      await invoke('video_delete_project', { id });
      if (currentProject.value?.id === id) currentProject.value = null;
      await loadProjects();
    } catch (e) {
      alert('删除失败: ' + e);
    }
  };

  return { projects, currentProject, loadProjects, createProject, selectProject, deleteProject };
}

export function useVideoMaterials() {
  const materials = ref<VideoMaterial[]>([]);
  const isUploadingMaterial = ref(false);

  const loadMaterials = async (projectId: string) => {
    try {
      materials.value = await invoke<VideoMaterial[]>('video_list_materials', { projectId });
    } catch (e) {
      console.error('Failed to load materials:', e);
    }
  };

  const uploadMaterial = async (projectId: string, type: string) => {
    const selected = await open({
      multiple: false,
      filters: [{
        name: type === 'image' ? 'Images' : (type === 'video' ? 'Videos' : 'Audios'),
        extensions: type === 'image' ? ['png', 'jpg', 'jpeg', 'webp'] : (type === 'video' ? ['mp4', 'mov', 'avi'] : ['mp3', 'wav', 'm4a'])
      }]
    });
    if (!selected || Array.isArray(selected)) return;

    isUploadingMaterial.value = true;
    try {
      await invoke('video_upload_material', {
        projectId,
        sourcePath: selected,
        materialType: type
      });
      await loadMaterials(projectId);
    } catch (e) {
      alert('上传失败: ' + e);
    } finally {
      isUploadingMaterial.value = false;
    }
  };

  const deleteMaterial = async (projectId: string, id: string) => {
    if (!confirm('确定要删除该素材吗？')) return;
    try {
      await invoke('video_delete_material', { id });
      await loadMaterials(projectId);
    } catch (e) {
      alert('删除失败: ' + e);
    }
  };

  return { materials, isUploadingMaterial, loadMaterials, uploadMaterial, deleteMaterial };
}
