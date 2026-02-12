<template>
  <Transition name="search-panel-slide">
    <div v-if="visible" class="search-results-panel">
      <!-- Header -->
      <div class="panel-header">
        <div class="header-left">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8"/>
            <line x1="21" y1="21" x2="16.65" y2="16.65"/>
          </svg>
          <span class="header-title">Search Results</span>
        </div>
        <button class="close-btn" @click="$emit('close')" title="Close panel">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"/>
            <line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </button>
      </div>

      <!-- Query display -->
      <div class="query-section" v-if="query">
        <span class="query-label">Query:</span>
        <span class="query-text">{{ query }}</span>
      </div>

      <!-- Content -->
      <div class="panel-body">
        <div class="search-content" v-html="renderedContent"></div>
      </div>
    </div>
  </Transition>
</template>

<script>
import { computed } from 'vue';
import MarkdownIt from 'markdown-it';

const md = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true
});

export default {
  name: 'SearchResultsPanel',
  props: {
    visible: {
      type: Boolean,
      default: false
    },
    query: {
      type: String,
      default: ''
    },
    content: {
      type: String,
      default: ''
    }
  },
  emits: ['close'],
  setup(props) {
    const renderedContent = computed(() => {
      if (!props.content) return '<p class="empty-state">No results yet.</p>';
      return md.render(props.content);
    });

    return {
      renderedContent
    };
  }
}
</script>

<style scoped>
.search-results-panel {
  position: absolute;
  top: 0;
  right: 0;
  width: 380px;
  max-width: 50%;
  height: 100%;
  background: #ffffff;
  border-left: 1px solid #c1c989;
  box-shadow: -4px 0 16px rgba(94, 108, 83, 0.12);
  display: flex;
  flex-direction: column;
  z-index: 100;
}

/* Header */
.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid #e8eadf;
  background: #f8f9f3;
  flex-shrink: 0;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #5e6c53;
}

.header-title {
  font-size: 13px;
  font-weight: 600;
  color: #5e6c53;
}

.close-btn {
  background: none;
  border: none;
  cursor: pointer;
  padding: 4px;
  color: #8a9a7e;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
}

.close-btn:hover {
  color: #5e6c53;
  background: rgba(94, 108, 83, 0.08);
}

/* Query section */
.query-section {
  padding: 10px 16px;
  border-bottom: 1px solid #e8eadf;
  background: #f4f5ee;
  flex-shrink: 0;
}

.query-label {
  font-size: 11px;
  font-weight: 600;
  color: #8a9a7e;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-right: 6px;
}

.query-text {
  font-size: 13px;
  color: #5e6c53;
  font-style: italic;
}

/* Body */
.panel-body {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  min-height: 0;
}

/* Markdown content styling */
.search-content :deep(h1),
.search-content :deep(h2),
.search-content :deep(h3) {
  color: #5e6c53;
  margin-top: 16px;
  margin-bottom: 8px;
  line-height: 1.3;
}

.search-content :deep(h1) { font-size: 18px; }
.search-content :deep(h2) { font-size: 16px; }
.search-content :deep(h3) { font-size: 14px; }

.search-content :deep(p) {
  font-size: 13px;
  color: #4a5a40;
  line-height: 1.7;
  margin-bottom: 10px;
}

.search-content :deep(a) {
  color: #7d9569;
  text-decoration: none;
  border-bottom: 1px solid rgba(125, 149, 105, 0.3);
  transition: border-color 0.15s;
}

.search-content :deep(a:hover) {
  border-bottom-color: #7d9569;
}

.search-content :deep(ul),
.search-content :deep(ol) {
  padding-left: 20px;
  margin-bottom: 10px;
}

.search-content :deep(li) {
  font-size: 13px;
  color: #4a5a40;
  line-height: 1.6;
  margin-bottom: 4px;
}

.search-content :deep(code) {
  background: #f0f1ea;
  padding: 2px 5px;
  border-radius: 3px;
  font-size: 12px;
  color: #5e6c53;
}

.search-content :deep(pre) {
  background: #f4f5ee;
  padding: 12px;
  border-radius: 6px;
  overflow-x: auto;
  margin-bottom: 12px;
}

.search-content :deep(pre code) {
  background: none;
  padding: 0;
}

.search-content :deep(blockquote) {
  border-left: 3px solid #c1c989;
  padding-left: 12px;
  margin-left: 0;
  color: #6b7c5f;
  font-style: italic;
}

.search-content :deep(.empty-state) {
  color: #8a9a7e;
  font-style: italic;
  text-align: center;
  padding: 40px 0;
}

/* Scrollbar */
.panel-body::-webkit-scrollbar {
  width: 6px;
}

.panel-body::-webkit-scrollbar-track {
  background: transparent;
}

.panel-body::-webkit-scrollbar-thumb {
  background: #c1c989;
  border-radius: 3px;
}

.panel-body::-webkit-scrollbar-thumb:hover {
  background: #a8b475;
}

/* Slide transition */
.search-panel-slide-enter-active {
  transition: transform 0.3s cubic-bezier(0.19, 1, 0.22, 1), opacity 0.2s ease;
}

.search-panel-slide-leave-active {
  transition: transform 0.25s ease-in, opacity 0.2s ease;
}

.search-panel-slide-enter-from {
  transform: translateX(100%);
  opacity: 0;
}

.search-panel-slide-leave-to {
  transform: translateX(100%);
  opacity: 0;
}
</style>
