<script setup lang="ts">
import { ref, computed } from 'vue';
import { UsersRound, Plus, Search, Trash2, X } from 'lucide-vue-next';

interface Person {
  id: string;
  name: string;
  empNo: string;
  department: string;
  role: string;
  status: '在职' | '休假' | '离职';
}

// 示例数据（前端内存，后续可对接后端接口）
const people = ref<Person[]>([
  { id: '1', name: '张伟', empNo: 'F1001', department: '生产一部', role: '操作工', status: '在职' },
  { id: '2', name: '李娜', empNo: 'F1002', department: '质检部', role: '质检员', status: '在职' },
  { id: '3', name: '王强', empNo: 'F1003', department: '仓储部', role: '仓管员', status: '休假' },
  { id: '4', name: '赵敏', empNo: 'F1004', department: '生产二部', role: '班组长', status: '在职' },
]);

const keyword = ref('');
const filtered = computed(() => {
  const k = keyword.value.trim().toLowerCase();
  if (!k) return people.value;
  return people.value.filter(p =>
    p.name.toLowerCase().includes(k) ||
    p.empNo.toLowerCase().includes(k) ||
    p.department.toLowerCase().includes(k) ||
    p.role.toLowerCase().includes(k)
  );
});

const onJob = computed(() => people.value.filter(p => p.status === '在职').length);

// 新增表单
const showAdd = ref(false);
const form = ref<Omit<Person, 'id'>>({ name: '', empNo: '', department: '', role: '', status: '在职' });

function openAdd() {
  form.value = { name: '', empNo: '', department: '', role: '', status: '在职' };
  showAdd.value = true;
}
function saveAdd() {
  if (!form.value.name.trim() || !form.value.empNo.trim()) {
    alert('请填写姓名和工号');
    return;
  }
  people.value.unshift({ id: crypto.randomUUID(), ...form.value });
  showAdd.value = false;
}
function removePerson(id: string) {
  people.value = people.value.filter(p => p.id !== id);
}

const statusClass = (s: Person['status']) =>
  s === '在职' ? 'bg-green-500/10 text-green-400 border-green-500/20'
  : s === '休假' ? 'bg-amber-500/10 text-amber-400 border-amber-500/20'
  : 'bg-gray-700/40 text-gray-400 border-gray-600/30';
</script>

<template>
  <div class="max-w-5xl mx-auto p-8 space-y-6">
    <!-- 统计 -->
    <div class="grid grid-cols-3 gap-4">
      <div class="bg-gray-900/60 border border-gray-800 rounded-2xl p-5">
        <div class="text-[11px] text-gray-500 uppercase tracking-wider mb-1">员工总数</div>
        <div class="text-2xl font-bold text-white">{{ people.length }}</div>
      </div>
      <div class="bg-gray-900/60 border border-gray-800 rounded-2xl p-5">
        <div class="text-[11px] text-gray-500 uppercase tracking-wider mb-1">在职</div>
        <div class="text-2xl font-bold text-green-400">{{ onJob }}</div>
      </div>
      <div class="bg-gray-900/60 border border-gray-800 rounded-2xl p-5">
        <div class="text-[11px] text-gray-500 uppercase tracking-wider mb-1">非在职</div>
        <div class="text-2xl font-bold text-gray-400">{{ people.length - onJob }}</div>
      </div>
    </div>

    <!-- 操作栏 -->
    <div class="flex items-center justify-between gap-4">
      <div class="relative flex-1 max-w-sm">
        <Search class="w-4 h-4 text-gray-500 absolute left-3 top-1/2 -translate-y-1/2" />
        <input v-model="keyword" type="text" placeholder="搜索姓名 / 工号 / 部门 / 岗位"
               class="w-full bg-gray-950 border border-gray-800 rounded-xl pl-9 pr-4 py-2.5 text-sm text-white placeholder-gray-600 focus:outline-none focus:border-orange-500" />
      </div>
      <button @click="openAdd"
              class="flex items-center gap-2 bg-orange-600 hover:bg-orange-500 text-white px-4 py-2.5 rounded-xl text-sm font-medium transition-all shadow-lg shadow-orange-900/20">
        <Plus class="w-4 h-4" /> 添加人员
      </button>
    </div>

    <!-- 列表 -->
    <div class="bg-gray-900/50 border border-gray-800 rounded-2xl overflow-hidden">
      <table class="w-full text-sm">
        <thead>
          <tr class="text-left text-[11px] uppercase tracking-wider text-gray-500 border-b border-gray-800 bg-gray-900/40">
            <th class="px-5 py-3 font-medium">姓名</th>
            <th class="px-5 py-3 font-medium">工号</th>
            <th class="px-5 py-3 font-medium">部门</th>
            <th class="px-5 py-3 font-medium">岗位</th>
            <th class="px-5 py-3 font-medium">状态</th>
            <th class="px-5 py-3 font-medium text-right">操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="p in filtered" :key="p.id" class="border-b border-gray-800/60 hover:bg-gray-800/30 transition-colors group">
            <td class="px-5 py-3 font-medium text-gray-100">{{ p.name }}</td>
            <td class="px-5 py-3 font-mono text-gray-400">{{ p.empNo }}</td>
            <td class="px-5 py-3 text-gray-300">{{ p.department }}</td>
            <td class="px-5 py-3 text-gray-300">{{ p.role }}</td>
            <td class="px-5 py-3">
              <span :class="['text-[11px] px-2 py-0.5 rounded-full border', statusClass(p.status)]">{{ p.status }}</span>
            </td>
            <td class="px-5 py-3 text-right">
              <button @click="removePerson(p.id)"
                      class="p-1.5 text-gray-600 hover:text-red-400 hover:bg-red-500/10 rounded-lg transition-all opacity-0 group-hover:opacity-100">
                <Trash2 class="w-4 h-4" />
              </button>
            </td>
          </tr>
          <tr v-if="filtered.length === 0">
            <td colspan="6" class="px-5 py-12 text-center text-gray-600">
              <UsersRound class="w-10 h-10 mx-auto mb-2 opacity-20" />
              <p class="text-sm">没有符合条件的人员</p>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- 新增弹窗 -->
    <div v-if="showAdd" class="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-6">
      <div class="bg-gray-900 border border-gray-800 rounded-2xl w-full max-w-md shadow-2xl">
        <header class="flex items-center justify-between px-5 py-4 border-b border-gray-800">
          <h3 class="font-bold text-white">添加人员</h3>
          <button @click="showAdd = false" class="text-gray-500 hover:text-white p-1 rounded-lg hover:bg-gray-800"><X class="w-5 h-5" /></button>
        </header>
        <div class="p-5 space-y-4">
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-xs text-gray-400 mb-1.5">姓名 <span class="text-red-400">*</span></label>
              <input v-model="form.name" type="text" class="w-full bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-orange-500" />
            </div>
            <div>
              <label class="block text-xs text-gray-400 mb-1.5">工号 <span class="text-red-400">*</span></label>
              <input v-model="form.empNo" type="text" class="w-full bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-orange-500" />
            </div>
            <div>
              <label class="block text-xs text-gray-400 mb-1.5">部门</label>
              <input v-model="form.department" type="text" class="w-full bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-orange-500" />
            </div>
            <div>
              <label class="block text-xs text-gray-400 mb-1.5">岗位</label>
              <input v-model="form.role" type="text" class="w-full bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-orange-500" />
            </div>
          </div>
          <div>
            <label class="block text-xs text-gray-400 mb-1.5">状态</label>
            <select v-model="form.status" class="w-full bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-orange-500">
              <option value="在职">在职</option>
              <option value="休假">休假</option>
              <option value="离职">离职</option>
            </select>
          </div>
        </div>
        <footer class="flex justify-end gap-3 px-5 py-4 border-t border-gray-800">
          <button @click="showAdd = false" class="px-4 py-2 text-sm text-gray-300 bg-gray-800 hover:bg-gray-700 rounded-lg">取消</button>
          <button @click="saveAdd" class="px-5 py-2 text-sm text-white bg-orange-600 hover:bg-orange-500 rounded-lg font-medium">保存</button>
        </footer>
      </div>
    </div>
  </div>
</template>
