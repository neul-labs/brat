<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { useRepoStore } from '../stores/repo';
import bratApi from '../api/brat';
import type { KbNote, KbSearchResult } from '../types/brat';
import LoadingSpinner from '../components/common/LoadingSpinner.vue';

const repoStore = useRepoStore();

const searchQuery = ref('');
const searchResults = ref<KbSearchResult[]>([]);
const productNotes = ref<KbNote[]>([]);
const architectureNotes = ref<KbNote[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const activeTab = ref<'product' | 'architecture' | 'search'>('product');
const selectedNote = ref<KbNote | null>(null);

const hasRepo = computed(() => !!repoStore.activeRepoId);

async function fetchNotes() {
  if (!repoStore.activeRepoId) return;
  loading.value = true;
  error.value = null;
  try {
    const [product, arch] = await Promise.all([
      bratApi.listProductNotes(repoStore.activeRepoId),
      bratApi.listArchitectureNotes(repoStore.activeRepoId),
    ]);
    productNotes.value = product;
    architectureNotes.value = arch;
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to load notes';
  } finally {
    loading.value = false;
  }
}

async function doSearch() {
  if (!repoStore.activeRepoId || !searchQuery.value.trim()) return;
  loading.value = true;
  error.value = null;
  try {
    const type = activeTab.value === 'search' ? undefined : activeTab.value;
    searchResults.value = await bratApi.kbSearch(repoStore.activeRepoId, searchQuery.value.trim(), type);
    activeTab.value = 'search';
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Search failed';
  } finally {
    loading.value = false;
  }
}

function selectNote(note: KbNote) {
  selectedNote.value = note;
}

function closeNote() {
  selectedNote.value = null;
}

function formatBody(body: string): string {
  return body
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
    .replace(/`([^`]+)`/g, '<code class="bg-gray-200 px-1 rounded">$1</code>')
    .replace(/\n/g, '<br />');
}

watch(() => repoStore.activeRepoId, () => {
  fetchNotes();
});

onMounted(() => {
  if (repoStore.activeRepoId) {
    fetchNotes();
  }
});
</script>

<template>
  <div class="space-y-4">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold text-gray-900">Knowledge Base</h1>
      <div class="flex items-center gap-2">
        <input
          v-model="searchQuery"
          type="text"
          placeholder="Search KB..."
          class="input w-64"
          @keydown.enter="doSearch"
        />
        <button
          @click="doSearch"
          class="btn-primary"
          :disabled="!searchQuery.trim() || !hasRepo"
        >
          Search
        </button>
      </div>
    </div>

    <div v-if="!hasRepo" class="card text-center text-gray-600">
      Select a repository to view the knowledge base.
    </div>

    <div v-else-if="error" class="bg-red-50 text-red-700 p-4 rounded-lg">
      {{ error }}
    </div>

    <div v-else>
      <!-- Tabs -->
      <div class="flex gap-1 border-b border-gray-200 mb-4">
        <button
          @click="activeTab = 'product'"
          :class="[
            'px-4 py-2 text-sm font-medium border-b-2 transition-colors',
            activeTab === 'product'
              ? 'border-blue-600 text-blue-600'
              : 'border-transparent text-gray-500 hover:text-gray-700'
          ]"
        >
          Product ({{ productNotes.length }})
        </button>
        <button
          @click="activeTab = 'architecture'"
          :class="[
            'px-4 py-2 text-sm font-medium border-b-2 transition-colors',
            activeTab === 'architecture'
              ? 'border-blue-600 text-blue-600'
              : 'border-transparent text-gray-500 hover:text-gray-700'
          ]"
        >
          Architecture ({{ architectureNotes.length }})
        </button>
        <button
          v-if="searchResults.length > 0"
          @click="activeTab = 'search'"
          :class="[
            'px-4 py-2 text-sm font-medium border-b-2 transition-colors',
            activeTab === 'search'
              ? 'border-blue-600 text-blue-600'
              : 'border-transparent text-gray-500 hover:text-gray-700'
          ]"
        >
          Search Results ({{ searchResults.length }})
        </button>
      </div>

      <div v-if="loading" class="flex items-center gap-2 text-gray-500 py-8">
        <LoadingSpinner size="sm" />
        <span>Loading...</span>
      </div>

      <!-- Product Notes -->
      <div v-else-if="activeTab === 'product'" class="space-y-2">
        <div
          v-for="note in productNotes"
          :key="note.slug"
          @click="selectNote(note)"
          class="card hover:bg-gray-50 cursor-pointer transition-colors"
        >
          <div class="flex items-center justify-between">
            <div class="font-medium">{{ note.title }}</div>
            <div class="flex gap-1">
              <span
                v-for="tag in note.tags"
                :key="tag"
                class="text-xs px-2 py-0.5 rounded bg-gray-100 text-gray-600"
              >
                {{ tag }}
              </span>
            </div>
          </div>
          <div class="text-xs text-gray-400 mt-1 font-mono">{{ note.slug }}</div>
        </div>
        <div v-if="productNotes.length === 0" class="card text-center text-gray-500">
          No product notes yet. Run bootstrap to generate them.
        </div>
      </div>

      <!-- Architecture Notes -->
      <div v-else-if="activeTab === 'architecture'" class="space-y-2">
        <div
          v-for="note in architectureNotes"
          :key="note.slug"
          @click="selectNote(note)"
          class="card hover:bg-gray-50 cursor-pointer transition-colors"
        >
          <div class="flex items-center justify-between">
            <div class="font-medium">{{ note.title }}</div>
            <div class="flex gap-1">
              <span
                v-for="tag in note.tags"
                :key="tag"
                class="text-xs px-2 py-0.5 rounded bg-gray-100 text-gray-600"
              >
                {{ tag }}
              </span>
            </div>
          </div>
          <div class="text-xs text-gray-400 mt-1 font-mono">{{ note.slug }}</div>
        </div>
        <div v-if="architectureNotes.length === 0" class="card text-center text-gray-500">
          No architecture notes yet. Run bootstrap to generate them.
        </div>
      </div>

      <!-- Search Results -->
      <div v-else-if="activeTab === 'search'" class="space-y-2">
        <div
          v-for="result in searchResults"
          :key="result.slug"
          class="card hover:bg-gray-50 cursor-pointer transition-colors"
        >
          <div class="flex items-center justify-between">
            <div class="font-medium">{{ result.title }}</div>
            <span class="text-xs px-2 py-0.5 rounded bg-blue-100 text-blue-800">
              {{ result.note_type }}
            </span>
          </div>
          <div class="text-xs text-gray-400 mt-1 font-mono">
            {{ result.slug }} — score: {{ result.score.toFixed(2) }}
          </div>
        </div>
        <div v-if="searchResults.length === 0" class="card text-center text-gray-500">
          No results found.
        </div>
      </div>
    </div>

    <!-- Note Detail Modal -->
    <div
      v-if="selectedNote"
      class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
      @click.self="closeNote"
    >
      <div class="bg-white rounded-lg shadow-xl max-w-2xl w-full max-h-[80vh] flex flex-col">
        <div class="p-4 border-b border-gray-200 flex items-center justify-between">
          <div>
            <h2 class="text-lg font-semibold">{{ selectedNote.title }}</h2>
            <div class="text-xs text-gray-400 font-mono">{{ selectedNote.slug }}</div>
          </div>
          <button @click="closeNote" class="text-gray-400 hover:text-gray-600 text-2xl leading-none">
            &times;
          </button>
        </div>
        <div class="p-4 overflow-y-auto flex-1">
          <div class="flex gap-1 mb-4">
            <span
              v-for="tag in selectedNote.tags"
              :key="tag"
              class="text-xs px-2 py-0.5 rounded bg-gray-100 text-gray-600"
            >
              {{ tag }}
            </span>
          </div>
          <div class="prose prose-sm max-w-none" v-html="formatBody(selectedNote.body)"></div>
        </div>
      </div>
    </div>
  </div>
</template>
