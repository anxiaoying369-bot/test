<script setup lang="ts">
import { ref, computed } from 'vue';
import { Boxes, Plus, Search, Trash2, X, AlertTriangle } from 'lucide-vue-next';

interface Item {
  id: string;
  name: string;
  sku: string;
  category: string;
  quantity: number;
  unit: string;
  safetyStock: number;
}

// 示例数据（前端内存，后续可对接后端接口）
const items = ref<Item[]>([
  { id: '1', name: '不锈钢螺栓 M8', sku: 'SKU-0001', category: '紧固件', quantity: 1200, unit: '个', safetyStock: 500 },
  { id: '2', name: '电机轴承 6204', sku: 'SKU-0002', category: '机械配件', quantity: 80, unit: '套', safetyStock: 100 },
  { id: '3', name: '润滑油 46#', sku: 'SKU-0003', category: '耗材', quantity: 36, unit: '桶', safetyStock: 20 },
  { id: '4', name: 'PVC 包装膜', sku: 'SKU-0004', category: '包装材料', quantity: 8, unit: '卷', safetyStock: 15 },
]);

const keyword = ref('');
const filtered = computed(() => {
  const k = keyword.value.trim().toLowerCase();
  if (!k) return items.value;
  return items.value.filter(i =>
    i.name.toLowerCase().includes(k) ||
    i.sku.toLowerCase().includes(k) ||
    i.category.toLowerCase().includes(k)
  );
});

const isLow = (i: Item) => i.quantity < i.safetyStock;
const lowCount = computed(() => items.value.filter(isLow).length);
const totalQty = computed(() => items.value.reduce((s, i) => s + i.quantity, 0));

// 新增表单
const showAdd = ref(false);
const form = ref<Omit<Item, 'id'>>({ name: '', sku: '', category: '', quantity: 0, unit: '个', safetyStock: 0 });

function openAdd() {
  form.value = { name: '', sku: '', category: '', quantity: 0, unit: '个', safetyStock: 0 };
  showAdd.value = true;
}
function saveAdd() {
  if (!form.value.name.trim() || !form.value.sku.trim()) {
    alert('请填写物料名称和 SKU');
    return;
  }
  items.value.unshift({ id: crypto.randomUUID(), ...form.value });
  showAdd.value = false;
}
function removeItem(id: string) {
  items.value = items.value.filter(i => i.id !== id);
}
</script>

<template>
  <div class="max-w-5xl mx-auto p-8 space-y-6">
    <!-- 统计 -->
    <div class="grid grid-cols-3 gap-4">
      <div class="bg-gray-900/60 border border-gray-800 rounded-2xl p-5">
        <div class="text-[11px] text-gray-500 uppercase tracking-wider mb-1">物料种类</div>
        <div class="text-2xl font-bold text-white">{{ items.length }}</div>
      </div>
      <div class="bg-gray-900/60 border border-gray-800 rounded-2xl p-5">
        <div class="text-[11px] text-gray-500 uppercase tracking-wider mb-1">库存总量</div>
        <div class="text-2xl font-bold text-blue-400">{{ totalQty }}</div>
      </div>
      <div class="bg-gray-900/60 border border-gray-800 rounded-2xl p-5">
        <div class="text-[11px] text-gray-500 uppercase tracking-wider mb-1 flex items-center gap-1">
          <AlertTriangle class="w-3 h-3 text-red-400" /> 低于安全库存
        </div>
        <div class="text-2xl font-bold" :class="lowCount > 0 ? 'text-red-400' : 'text-gray-400'">{{ lowCount }}</div>
      </div>
    </div>

    <!-- 操作栏 -->
    <div class="flex items-center justify-between gap-4">
      <div class="relative flex-1 max-w-sm">
        <Search class="w-4 h-4 text-gray-500 absolute left-3 top-1/2 -translate-y-1/2" />
        <input v-model="keyword" type="text" placeholder="搜索物料名称 / SKU / 分类"
               class="w-full bg-gray-950 border border-gray-800 rounded-xl pl-9 pr-4 py-2.5 text-sm text-white placeholder-gray-600 focus:outline-none focus:border-orange-500" />
      </div>
      <button @click="openAdd"
              class="flex items-center gap-2 bg-orange-600 hover:bg-orange-500 text-white px-4 py-2.5 rounded-xl text-sm font-medium transition-all shadow-lg shadow-orange-900/20">
        <Plus class="w-4 h-4" /> 添加物料
      </button>
    </div>

    <!-- 列表 -->
    <div class="bg-gray-900/50 border border-gray-800 rounded-2xl overflow-hidden">
      <table class="w-full text-sm">
        <thead>
          <tr class="text-left text-[11px] uppercase tracking-wider text-gray-500 border-b border-gray-800 bg-gray-900/40">
            <th class="px-5 py-3 font-medium">物料名称</th>
            <th class="px-5 py-3 font-medium">SKU</th>
            <th class="px-5 py-3 font-medium">分类</th>
            <th class="px-5 py-3 font-medium text-right">库存数量</th>
            <th class="px-5 py-3 font-medium text-right">安全库存</th>
            <th class="px-5 py-3 font-medium text-right">操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="i in filtered" :key="i.id" class="border-b border-gray-800/60 hover:bg-gray-800/30 transition-colors group">
            <td class="px-5 py-3 font-medium text-gray-100">
              {{ i.name }}
              <span v-if="isLow(i)" class="ml-2 inline-flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded-full bg-red-500/10 text-red-400 border border-red-500/20">
                <AlertTriangle class="w-2.5 h-2.5" /> 缺货
              </span>
            </td>
            <td class="px-5 py-3 font-mono text-gray-400">{{ i.sku }}</td>
            <td class="px-5 py-3 text-gray-300">{{ i.category }}</td>
            <td class="px-5 py-3 text-right font-mono" :class="isLow(i) ? 'text-red-400 font-bold' : 'text-gray-100'">
              {{ i.quantity }} {{ i.unit }}
            </td>
            <td class="px-5 py-3 text-right font-mono text-gray-500">{{ i.safetyStock }} {{ i.unit }}</td>
            <td class="px-5 py-3 text-right">
              <button @click="removeItem(i.id)"
                      class="p-1.5 text-gray-600 hover:text-red-400 hover:bg-red-500/10 rounded-lg transition-all opacity-0 group-hover:opacity-100">
                <Trash2 class="w-4 h-4" />
              </button>
            </td>
          </tr>
          <tr v-if="filtered.length === 0">
            <td colspan="6" class="px-5 py-12 text-center text-gray-600">
              <Boxes class="w-10 h-10 mx-auto mb-2 opacity-20" />
              <p class="text-sm">没有符合条件的物料</p>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- 新增弹窗 -->
    <div v-if="showAdd" class="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-6">
      <div class="bg-gray-900 border border-gray-800 rounded-2xl w-full max-w-md shadow-2xl">
        <header class="flex items-center justify-between px-5 py-4 border-b border-gray-800">
          <h3 class="font-bold text-white">添加物料</h3>
          <button @click="showAdd = false" class="text-gray-500 hover:text-white p-1 rounded-lg hover:bg-gray-800"><X class="w-5 h-5" /></button>
        </header>
        <div class="p-5 space-y-4">
          <div class="grid grid-cols-2 gap-3">
            <div class="col-span-2">
              <label class="block text-xs text-gray-400 mb-1.5">物料名称 <span class="text-red-400">*</span></label>
              <input v-model="form.name" type="text" class="w-full bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-orange-500" />
            </div>
            <div>
              <label class="block text-xs text-gray-400 mb-1.5">SKU <span class="text-red-400">*</span></label>
              <input v-model="form.sku" type="text" class="w-full bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-orange-500" />
            </div>
            <div>
              <label class="block text-xs text-gray-400 mb-1.5">分类</label>
              <input v-model="form.category" type="text" class="w-full bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-orange-500" />
            </div>
            <div>
              <label class="block text-xs text-gray-400 mb-1.5">库存数量</label>
              <input v-model.number="form.quantity" type="number" min="0" class="w-full bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-orange-500" />
            </div>
            <div>
              <label class="block text-xs text-gray-400 mb-1.5">单位</label>
              <input v-model="form.unit" type="text" class="w-full bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-orange-500" />
            </div>
            <div class="col-span-2">
              <label class="block text-xs text-gray-400 mb-1.5">安全库存</label>
              <input v-model.number="form.safetyStock" type="number" min="0" class="w-full bg-gray-950 border border-gray-800 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-orange-500" />
            </div>
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
