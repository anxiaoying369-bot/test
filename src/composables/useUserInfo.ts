import { ref, computed, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export interface AccountItem {
  id: string;
  platform: string;
  name: string;
  status: string;
  meta: { user_id?: string; nickname?: string; avatar?: string };
}

// 字段对齐 scripts/douyin_get_user_info.py 的输出（空字段会被脚本省略）
export interface DouyinUserInfo {
  sec_uid?: string;
  uid?: string;
  short_id?: string;
  unique_id?: string;     // 抖音号
  nickname?: string;
  signature?: string;
  avatar_url?: string;
  follower_count?: number;
  following_count?: number;
  aweme_count?: number;
  favoriting_count?: number;
  total_favorited?: number;
  gender?: number;
  location?: string;
  school?: string;
  custom_verify?: string;
  enterprise_verify_reason?: string;
  source_strategy?: string;
}

export function useUserInfo() {
  const accounts = ref<AccountItem[]>([]);
  const selectedAccount = ref('');
  const input = ref('');
  const loading = ref(false);
  const error = ref('');
  const user = ref<DouyinUserInfo | null>(null);

  const douyinAccounts = computed(() => accounts.value.filter(a => a.platform === 'douyin'));
  const partial = ref(false);

  async function loadAccounts() {
    try {
      const res = await invoke('list_accounts', { platform: null }) as AccountItem[];
      accounts.value = res;
      // 默认选中第一个抖音账号
      if (!selectedAccount.value && douyinAccounts.value.length > 0) {
        selectedAccount.value = douyinAccounts.value[0].name;
      }
    } catch (e) {
      console.error('加载账号失败:', e);
    }
  }

  async function fetchUserInfo() {
    if (!selectedAccount.value) {
      error.value = '请选择一个抖音账号（用于提供 Cookie）';
      return;
    }
    const raw = input.value.trim();
    if (!raw) {
      error.value = '请输入用户 ID / 抖音号 / 分享链接 / 主页链接';
      return;
    }

    error.value = '';
    partial.value = false;
    loading.value = true;
    user.value = null;

    try {
      // douyin_get_user_info.py 自行识别 sec_uid / uid / 抖音号 / 短链 / 主页URL，无需预解析
      const res = await invoke('fetch_douyin_user_info', {
        accountName: selectedAccount.value,
        userId: raw,
      }) as { status: string; user?: DouyinUserInfo; error?: string };

      if ((res.status === 'ok' || res.status === 'partial') && res.user) {
        user.value = res.user;
        partial.value = res.status === 'partial';
        if (partial.value && !res.user.nickname) {
          error.value = '仅获取到部分信息（可能 Cookie 已失效，或页面未完全加载）。';
        }
      } else {
        error.value = res.error || '获取用户信息失败。';
      }
    } catch (e: any) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  function reset() {
    user.value = null;
    error.value = '';
    partial.value = false;
    input.value = '';
  }

  onMounted(loadAccounts);

  return {
    accounts, selectedAccount, input, loading, error, user, partial,
    douyinAccounts,
    loadAccounts, fetchUserInfo, reset,
  };
}
