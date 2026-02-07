<template>
  <div class="live-canvas">
    <!-- Edit/Done Button -->
    <div class="canvas-toolbar">
      <button 
        v-if="!isEditing" 
        @click="startEditing" 
        class="canvas-btn"
        title="Edit Source"
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>
          <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
        </svg>
        <span>Edit</span>
      </button>
      <button 
        v-else 
        @click="stopEditing" 
        class="canvas-btn done"
        title="Done Editing"
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="20 6 9 17 4 12"/>
        </svg>
        <span>Done</span>
      </button>
    </div>

    <!-- View Mode (Rendered Markdown) -->
    <div 
        v-if="!isEditing && !isEmpty"
        ref="viewerRef"
        class="markdown-body"
        v-html="renderedContent"
    ></div>
    
    <!-- Empty State -->
    <div
      v-if="!isEditing && isEmpty"
      class="empty-hint"
    >
      Empty document
    </div>

    <!-- Edit Mode (Textarea) -->
    <div v-if="isEditing" class="editor-container">
      <textarea
          ref="editorRef"
          v-model="localContent"
          @keydown.esc="stopEditing"
          class="editor-compact"
      ></textarea>
    </div>
  </div>
</template>

<script setup>
import { computed, ref, watch, nextTick, onMounted } from 'vue';
import MarkdownIt from 'markdown-it';
import DiffMatchPatch from 'diff-match-patch';
import mermaid from 'mermaid';

const props = defineProps({
  content: {
    type: String,
    default: ''
  },
  docId: {
    type: String,
    default: ''
  },
  isRecording: {
    type: Boolean,
    default: false
  }
});

const emit = defineEmits(['update:content']);

const md = new MarkdownIt({
    html: true,
    linkify: true,
    typographer: true,
});

// Custom fence renderer for mermaid
const defaultFence = md.renderer.rules.fence || function (tokens, idx, options, env, self) {
  return '<pre><code class="hljs">' + md.utils.escapeHtml(tokens[idx].content) + '</code></pre>';
};

md.renderer.rules.fence = function (tokens, idx, options, env, self) {
  const token = tokens[idx];
  const info = token.info ? token.info.trim() : '';
  if (info === 'mermaid') {
    return `<div class="mermaid">${token.content}</div>`;
  }
  return defaultFence(tokens, idx, options, env, self);
};

const isEditing = ref(false);
const localContent = ref(props.content);
const editorRef = ref(null);
const viewerRef = ref(null);
const prevPlainText = ref('');
const ignoreNextHighlight = ref(false);

const renderedContent = computed(() => {
  if (!props.content) return '';

  // Function to process tabs in non-code parts
  const processTabs = (text) => {
    // Split by code fences (```...```) and inline code (`...`)
    // We utilize a simple regex that captures the code blocks
    const parts = text.split(/(```[\s\S]*?```|`[^`\n]+`)/g);
    
    return parts.map(part => {
      // If part starts with backtick, it's code - return as is
      if (part.startsWith('`')) {
        return part;
      }
      // Otherwise, replace literal \t and real tabs with 4 spaces to ensure Markdown nesting
      // Using &emsp; causes it to be treated as text, not structure. 4 spaces triggers nested lists.
      return part.replace(/(\\t|\t)/g, '    ');
    }).join('');
  };

  const processed = processTabs(props.content);
  return md.render(processed);
});
const isEmpty = computed(() => (props.content ?? '').trim().length === 0);

// Sync props to local state when backend updates (if not editing)
watch(() => props.content, (newVal) => {
    if (!isEditing.value) {
        localContent.value = newVal;
    }
});

// Debounce emit for local content changes
let debounceTimer = null;
watch(localContent, (newVal) => {
    if (!isEditing.value) return;
    
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
        if (newVal !== props.content) {
            emit('update:content', newVal);
        }
    }, 500); // 500ms debounce
});

const isInitialLoad = ref(true);

// Reset initial load flag when switched to a different document
watch(() => props.docId, () => {
  // Ensure we exit edit mode when switching documents
  if (isEditing.value) {
      stopEditing();
  }
  isInitialLoad.value = true;
  ignoreNextHighlight.value = false;
});

function renderAndHighlight() {
  if (!viewerRef.value) return;

  // 1) Build plain text
  const newPlain = viewerRef.value.textContent ?? '';
  
  // If this is the initial load of a document OR we are not recording, don't show any highlights
  if (isInitialLoad.value || !props.isRecording) {
    prevPlainText.value = newPlain;
    isInitialLoad.value = false;
    
    // Still need to render Mermaid
    nextTick(() => {
      const mermaidNodes = viewerRef.value.querySelectorAll('.mermaid');
      if (mermaidNodes.length > 0) {
        mermaid.run({ nodes: mermaidNodes }).catch(err => console.error('Mermaid error:', err));
      }
    });
    return;
  }

  const oldPlain = prevPlainText.value ?? '';

  // 2) Compute diff ranges
  const dmp = new DiffMatchPatch();
  const diffs = dmp.diff_main(oldPlain, newPlain);
  dmp.diff_cleanupEfficiency(diffs);

  const ranges = [];
  let newIdx = 0;
  for (let i = 0; i < diffs.length; i++) {
    const [op, text] = diffs[i];
    if (op === 0) {
      newIdx += text.length;
      continue;
    }
    if (op === -1) continue;
    if (op === 1) {
      const prev = diffs[i - 1];
      const next = diffs[i + 1];
      const isMod = (prev && prev[0] === -1) || (next && next[0] === -1);
      ranges.push({
        start: newIdx,
        end: newIdx + text.length,
        kind: isMod ? 'mod' : 'add'
      });
      newIdx += text.length;
    }
  }

  // 3) Apply highlights
  try {
    applyRangesToDom(viewerRef.value, ranges);
  } catch (err) {
    console.error('Highlight error:', err);
  }

  // 4) Update prev
  prevPlainText.value = newPlain;

  // 5) Render Mermaid diagrams
  nextTick(() => {
    const mermaidNodes = viewerRef.value.querySelectorAll('.mermaid');
    if (mermaidNodes.length > 0) {
      mermaid.run({ nodes: mermaidNodes }).catch(err => console.error('Mermaid error:', err));
    }
  });
}

// Render markdown and apply character-level highlights
watch(
  () => props.content,
  async () => {
    if (isEditing.value) return;
    
    // If flag is set, treat this update as the new baseline (no highlight)
    if (ignoreNextHighlight.value) {
        ignoreNextHighlight.value = false;
        await nextTick();
        if (viewerRef.value) {
            prevPlainText.value = viewerRef.value.textContent ?? '';
        }
        // Still render Mermaid if needed
        nextTick(() => {
            const mermaidNodes = viewerRef.value?.querySelectorAll('.mermaid');
            if (mermaidNodes?.length > 0) {
                mermaid.run({ nodes: mermaidNodes }).catch(err => console.error(err));
            }
        });
        return;
    }

    await nextTick();
    renderAndHighlight();
  },
  { immediate: true }
);

// Force re-render when switching from edit -> view
watch(
  () => isEditing.value,
  async (editing) => {
    if (!editing) {
      // Treat returning from edit mode as an initial load to avoid full-page highlights
      isInitialLoad.value = true;
      await nextTick();
      renderAndHighlight();
    }
  }
);

function getTextNodes(root) {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      if (!node.nodeValue) return NodeFilter.FILTER_REJECT;
      if (node.nodeValue.length === 0) return NodeFilter.FILTER_REJECT;
      
      // Check if any ancestor is a mermaid block
      let parent = node.parentElement;
      while (parent && parent !== root) {
        if (parent.classList.contains('mermaid')) {
          return NodeFilter.FILTER_REJECT;
        }
        parent = parent.parentElement;
      }
      
      return NodeFilter.FILTER_ACCEPT;
    }
  });
  const nodes = [];
  let n;
  while ((n = walker.nextNode())) nodes.push(n);
  return nodes;
}

function applyRangesToDom(root, ranges) {
  if (!ranges || ranges.length === 0) return;

  // merge overlapping ranges of same kind to reduce DOM churn
  const sorted = ranges
    .filter(r => r.end > r.start)
    .sort((a, b) => a.start - b.start);

  const merged = [];
  for (const r of sorted) {
    const last = merged[merged.length - 1];
    if (last && r.start <= last.end && r.kind === last.kind) {
      last.end = Math.max(last.end, r.end);
    } else {
      merged.push({ ...r });
    }
  }

  const textNodes = getTextNodes(root);
  let cursor = 0;

  for (const node of textNodes) {
    const text = node.nodeValue ?? '';
    const nodeStart = cursor;
    const nodeEnd = cursor + text.length;

    // ranges overlapping this node
    const overlaps = merged
      .filter(r => r.start < nodeEnd && r.end > nodeStart)
      .map(r => ({
        start: Math.max(0, r.start - nodeStart),
        end: Math.min(text.length, r.end - nodeStart),
        kind: r.kind
      }))
      .sort((a, b) => b.start - a.start); // process from end to avoid offset shifts

    let currentNode = node;
    for (const o of overlaps) {
      if (o.end <= o.start) continue;

      // Split at end, then at start
      const after = currentNode.splitText(o.end);
      const mid = currentNode.splitText(o.start);

      const span = document.createElement('span');
      span.className = o.kind === 'add' ? 'hl-add' : 'hl-mod';
      span.appendChild(mid);
      after.parentNode.insertBefore(span, after);

      currentNode = currentNode; // continue splitting earlier parts of currentNode
    }

    cursor = nodeEnd;
  }
}

const startEditing = async () => {
    isEditing.value = true;
    await nextTick();
    editorRef.value?.focus();
};

const stopEditing = () => {
    isEditing.value = false;
    
    // Clear system selection artifacts with a small delay to ensure DOM update
    // Force blur to remove focus ring/selection context
    if (editorRef.value) {
        editorRef.value.blur();
    }
    
    // Immediate clear
    if (window.getSelection) {
        window.getSelection().removeAllRanges();
    }
    
    // Delayed clear to catch any render-swapping artifacts
    setTimeout(() => {
        if (window.getSelection) {
            window.getSelection().removeAllRanges();
        }
    }, 50);
    
    if (localContent.value !== props.content) {
        // Flag to ignore highlighting the update we just triggered ourselves
        ignoreNextHighlight.value = true;
        emit('update:content', localContent.value);
    }
};

onMounted(() => {
    mermaid.initialize({ startOnLoad: false, theme: 'default' });
});

</script>

<style scoped>
.live-canvas {
  position: relative;
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.markdown-body {
  cursor: text;
  user-select: text; /* Re-enable selection for editor content */
  width: 100%;
  flex: 1;
  overflow-y: auto;
  box-sizing: border-box;
  padding: 1rem;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Helvetica', 'Arial', sans-serif;
  font-size: 0.95rem;
  line-height: 1.6;
  color: #5e6c53;
}

/* Markdown styling */
.markdown-body :deep(h1),
.markdown-body :deep(h2),
.markdown-body :deep(h3),
.markdown-body :deep(h4),
.markdown-body :deep(h5),
.markdown-body :deep(h6) {
  margin-top: 1.5em;
  margin-bottom: 0.5em;
  font-weight: 600;
  color: #3d4a33;
}

.markdown-body :deep(h1) { font-size: 2rem; }
.markdown-body :deep(h2) { font-size: 1.6rem; }
.markdown-body :deep(h3) { font-size: 1.3rem; }
.markdown-body :deep(h4) { font-size: 1.1rem; }
.markdown-body :deep(h5) { font-size: 1rem; }
.markdown-body :deep(h6) { font-size: 0.95rem; }

.markdown-body :deep(p) {
  margin-bottom: 1em;
}

.markdown-body :deep(ul),
.markdown-body :deep(ol) {
  margin-bottom: 1em;
  padding-left: 2em;
}

.markdown-body :deep(code) {
  background: #f0f2e8;
  padding: 0.2em 0.4em;
  border-radius: 3px;
  font-family: 'Monaco', 'Menlo', 'Consolas', monospace;
  font-size: 0.9em;
}

.markdown-body :deep(pre) {
  background: #f0f2e8;
  padding: 1em;
  border-radius: 5px;
  overflow-x: auto;
  margin-bottom: 1em;
}

.markdown-body :deep(pre code) {
  background: none;
  padding: 0;
}

.markdown-body :deep(blockquote) {
  border-left: 4px solid #c1c989;
  padding-left: 1em;
  margin-left: 0;
  color: #7d9569;
  font-style: italic;
}

.canvas-toolbar {
  position: absolute;
  top: 12px;
  right: 12px;
  z-index: 10;
}

.canvas-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 12px;
  background: rgba(255, 255, 255, 0.95);
  border: 1px solid #c1c989;
  border-radius: 6px;
  color: #5e6c53;
  cursor: pointer;
  font-size: 13px;
  transition: all 0.15s;
  box-shadow: 0 2px 8px rgba(94, 108, 83, 0.1);
}

.canvas-btn:hover {
  background: #e0e7a0;
  border-color: #7d9569;
  color: #3d4a33;
}

.canvas-btn.done {
  background: #7d9569;
  border-color: #7d9569;
  color: #ffffff;
}

.canvas-btn.done:hover {
  background: #6a8358;
}

.empty-hint {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
  user-select: none;
  font-size: 0.95rem;
  color: #c1c989;
}

.editor-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.editor-compact {
  font-size: 0.95rem;
  line-height: 1.35;
  width: 100%;
  height: 100%;
  flex: 1;
  box-sizing: border-box;
  border: none;
  outline: none;
  resize: none;
  padding: 1rem;
  background: #ffffff;
  color: #5e6c53;
  user-select: text; /* Re-enable selection for textarea */
}

.editor-compact:focus {
  outline: none;
  border: none;
}

.markdown-body, .editor-compact {
  tab-size: 4;
}

/* Character-level highlight (temporary, elegant) */
@keyframes hl-fade {
  0% { background-color: var(--hl-bg); }
  100% { background-color: transparent; }
}

.markdown-body :deep(.hl-add) {
  --hl-bg: rgba(210, 244, 224, 0.75); /* pale green */
  animation: hl-fade 3s ease-out;
  border-radius: 2px;
}

.markdown-body :deep(.hl-mod) {
  --hl-bg: rgba(255, 243, 205, 0.75); /* pale yellow */
  animation: hl-fade 3s ease-out;
  border-radius: 2px;
}
</style>
