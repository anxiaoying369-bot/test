import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

export interface KBChunk {
  text: string;
  chunk_id: number;
  source: string;
}

export function useKnowledgeBase() {
  const files = ref<string[]>([]);
  const isLoading = ref(false);
  const isUploading = ref(false);
  const errorMsg = ref('');
  const successMsg = ref('');

  // 详情相关
  const isDetailModalOpen = ref(false);
  const isDetailLoading = ref(false);
  const selectedFileName = ref('');
  const fileChunks = ref<KBChunk[]>([]);

  async function loadFiles() {
    isLoading.value = true;
    errorMsg.value = '';
    try {
      const res = await invoke('list_kb_files') as string[];
      files.value = res;
    } catch (e: any) {
      console.error('加载知识库失败:', e);
      errorMsg.value = String(e);
    } finally {
      isLoading.value = false;
    }
  }

  async function handleUpload() {
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'Documents',
          extensions: ['txt', 'pdf', 'json', 'docx', 'xlsx', 'xls']
        }]
      });

      if (selected && typeof selected === 'string') {
        isUploading.value = true;
        errorMsg.value = '';
        successMsg.value = '';
        
        const res = await invoke('add_to_kb', { filePath: selected }) as any;
        if (res.status === 'success') {
          successMsg.value = `上传成功: 增加了 ${res.chunks_added} 个知识切片`;
          await loadFiles();
        } else {
          errorMsg.value = res.error || '上传失败';
        }
      }
    } catch (e: any) {
      console.error('上传失败:', e);
      errorMsg.value = String(e);
    } finally {
      isUploading.value = false;
    }
  }

  async function deleteFile(filename: string) {
    if (!confirm(`确定要从知识库中删除文件 "${filename}" 吗？`)) return;
    
    try {
      await invoke('delete_kb_file', { filename });
      successMsg.value = '文件已删除';
      await loadFiles();
      if (selectedFileName.value === filename) {
        isDetailModalOpen.value = false;
      }
    } catch (e: any) {
      console.error('删除失败:', e);
      errorMsg.value = String(e);
    }
  }

  async function showDetails(filename: string) {
    selectedFileName.value = filename;
    isDetailModalOpen.value = true;
    isDetailLoading.value = true;
    fileChunks.value = [];
    
    try {
      const res = await invoke('get_kb_file_details', { filename }) as KBChunk[];
      fileChunks.value = res;
    } catch (e: any) {
      console.error('获取详情失败:', e);
      errorMsg.value = String(e);
    } finally {
      isDetailLoading.value = false;
    }
  }

  onMounted(loadFiles);

  return {
    files, isLoading, isUploading, errorMsg, successMsg,
    isDetailModalOpen, isDetailLoading, selectedFileName, fileChunks,
    loadFiles, handleUpload, deleteFile, showDetails,
  };
}
