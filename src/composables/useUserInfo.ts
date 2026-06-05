import { ref, computed, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export interface AccountItem {
  id: string;
  platform: string;
  name: string;
  status: string;
  meta: { user_id?: string; nickname?: string; avatar?: string };
}

// 与 src-tauri models::UserCard 对齐
export interface UserCard {
  sec_uid: string;
  uid: string;
  unique_id: string;     // 抖音号
  nickname: string;
  avatar_url: string;
  signature: string;
  follower_count: number;
  following_count: number;
  total_favorited: number;
  aweme_count: number;
  ip_location: string;
  updated_at: string;
}

export function useUserInfo() {
  const accounts = ref<AccountItem[]>([]);
  const selectedAccount = ref('');
  const input = ref('');
  const loading = ref(false);
  const error = ref('');
  const cards = ref<UserCard[]>([]);
  const refreshingSecUid = ref('');

  const douyinAccounts = computed(() => accounts.value.filter(a => a.platform === 'douyin'));

  async function loadAccounts() {
    try {
      const res = await invoke('list_accounts', { platform: null }) as AccountItem[];
      accounts.value = res;
      if (!selectedAccount.value && douyinAccounts.value.length > 0) {
        selectedAccount.value = douyinAccounts.value[0].name;
      }
    } catch (e) {
      console.error('加载账号失败:', e);
    }
  }

  async function loadCards() {
    try {
      cards.value = await invoke('list_user_cards') as UserCard[];
    } catch (e) {
      console.error('加载用户库失败:', e);
    }
  }

  async function queryUser() {
    if (!selectedAccount.value) {
      error.value = '请选择一个抖音账号（用于提供 Cookie）';
      return;
    }
    const raw = input.value.trim();
    if (!raw) {
      error.value = '请输入 sec_uid 或主页链接';
      return;
    }

    error.value = '';
    loading.value = true;
    try {
      const card = await invoke('query_and_save_user', {
        accountName: selectedAccount.value,
        userId: raw,
      }) as UserCard;
      input.value = '';
      // 更新本地列表（去重后置顶）
      cards.value = [card, ...cards.value.filter(c => c.sec_uid !== card.sec_uid)];
    } catch (e: any) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function refreshCard(secUid: string) {
    if (!selectedAccount.value) {
      error.value = '请先选择一个抖音账号';
      return;
    }
    refreshingSecUid.value = secUid;
    error.value = '';
    try {
      const card = await invoke('refresh_user_card', {
        accountName: selectedAccount.value,
        secUid,
      }) as UserCard;
      cards.value = cards.value.map(c => (c.sec_uid === secUid ? card : c));
    } catch (e: any) {
      error.value = String(e);
    } finally {
      refreshingSecUid.value = '';
    }
  }

  async function deleteCard(secUid: string) {
    try {
      await invoke('delete_user_card', { secUid });
      cards.value = cards.value.filter(c => c.sec_uid !== secUid);
    } catch (e: any) {
      error.value = String(e);
    }
  }

  onMounted(async () => {
    await Promise.all([loadAccounts(), loadCards()]);
  });

  return {
    accounts, selectedAccount, input, loading, error, cards, refreshingSecUid,
    douyinAccounts,
    loadAccounts, loadCards, queryUser, refreshCard, deleteCard,
  };
}
