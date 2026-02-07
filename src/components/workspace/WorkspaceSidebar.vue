<template>
  <div class="file-explorer">
    <div class="explorer-header" data-tauri-drag-region>
      <button @click="$emit('navigate-back')" class="icon-btn back-btn" title="Back to Workspaces">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M19 12H5"/>
          <path d="M12 19l-7-7 7-7"/>
        </svg>
      </button>
      <img src="@/assets/logo.svg" alt="Creek" class="brand-logo" />
      <span class="brand-text">Creek <span class="brand-by">by</span> Monticule</span>
      <div class="explorer-actions">
        <button @click="triggerFileUpload" class="icon-btn" title="Upload Files">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="17 8 12 3 7 8"/>
            <line x1="12" y1="3" x2="12" y2="15"/>
          </svg>
        </button>
        <button @click="$emit('refresh')" class="icon-btn" title="Refresh Explorer">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M23 4v6h-6"/>
            <path d="M1 20v-6h6"/>
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>
          </svg>
        </button>
      </div>
      <input 
        ref="fileInput" 
        type="file" 
        multiple 
        @change="handleFileUpload" 
        style="display: none;"
      />
    </div>


    <div 
      class="file-list"
      @contextmenu.prevent="handleListContextMenu"
      @drop.prevent="handleDrop"
      @dragover.prevent="handleDragOver"
      @dragleave="handleDragLeave"
      :class="{ 'drag-over': isDraggingOver }"
    >
      <div 
        v-for="item in flatItems" 
        :key="item.uniqueKey" 
        class="list-item"
        :class="{ 
          'selected': selectedFiles.includes(item.id),
          'primary-selected': selectedFile === item.id 
        }"
        @click="handleItemClick($event, item)"
        @contextmenu.prevent.stop="handleItemContextMenu($event, item)"
      >
        <div class="item-content">
          <template v-if="item.type === 'recording'">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="file-icon recording">
              <circle cx="12" cy="12" r="10"/>
              <circle cx="12" cy="12" r="3"/>
            </svg>
          </template>
          <template v-else-if="item.isDirectory">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" class="folder-icon">
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
            </svg>
          </template>
          <template v-else>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="file-icon">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
              <polyline points="14 2 14 8 20 8"/>
            </svg>
          </template>
          
          <span class="item-name" :title="item.name">{{ item.name }}</span>
          
          <div class="item-actions">
            <button class="action-btn" @click.stop="exportSingleItem(item)" title="Export">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                <polyline points="7 10 12 15 17 10"/>
                <line x1="12" y1="15" x2="12" y2="3"/>
              </svg>
            </button>
            <button class="action-btn delete" @click.stop="deleteSingleItem(item)" title="Delete">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="18" y1="6" x2="6" y2="18"></line>
                <line x1="6" y1="6" x2="18" y2="18"></line>
              </svg>
            </button>
          </div>
        </div>
      </div>

      <div v-if="flatItems.length === 0" class="empty-list">
        <span>No files</span>
      </div>
    </div>

    <!-- Context Menu -->
    <div 
      v-if="contextMenu.show" 
      class="context-menu"
      :style="{ top: contextMenu.y + 'px', left: contextMenu.x + 'px' }"
      @click.stop
    >
      <div v-if="contextMenu.type === 'item'" class="context-menu-items">
        <div class="context-menu-item" @click.stop="renameFile">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>
            <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
          </svg>
          <span>Rename</span>
        </div>
        <div class="context-menu-item" @click.stop="exportFile">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="7 10 12 15 17 10"/>
            <line x1="12" y1="15" x2="12" y2="3"/>
          </svg>
          <span>Export</span>
        </div>
        <div class="context-menu-item danger" @click.stop.prevent="deleteFiles">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="3 6 5 6 21 6"/>
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
          </svg>
          <span>Delete</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
import { ref, computed, onMounted, onUnmounted } from 'vue';

export default {
  name: 'WorkspaceSidebar',
  props: {
    files: {
      type: Array,
      required: true
    },
    recordings: {
      type: Array,
      required: true
    },
    selectedFiles: {
      type: Array,
      default: () => []
    },
    selectedFile: {
      type: String,
      default: null
    }
  },
  emits: [
    'update:selectedFiles',
    'update:selectedFile',
    'select-file',
    'select-recording',
    'refresh',
    'upload-files',
    'delete-files',
    'rename-file',
    'export-files',
    'drop-files',
    'navigate-back'
  ],
  setup(props, { emit }) {
    const fileInput = ref(null);
    const isDraggingOver = ref(false);
    
    // Context menu state internal to sidebar UI
    const contextMenu = ref({
      show: false,
      x: 0,
      y: 0,
      type: null,
      target: null
    });

    const flatItems = computed(() => {
      // Create a unified list of items.
      const recs = props.recordings.map(r => {
        // Handle both string (legacy) and object formats
        const isObj = typeof r === 'object' && r !== null;
        const name = isObj ? r.name : r;
        const id = isObj ? r.id : r;
        
        return {
          name: name,
          id: id, // Logical identifier
          type: 'recording',
          uniqueKey: 'rec-' + id,
          isDirectory: false
        };
      });

      const fls = props.files.map(f => ({
        ...f,
        id: f.name, // For simple files, name is the id
        type: 'file',
        uniqueKey: 'file-' + f.name
      }));
      
      // Merge them. Recordings first, then files.
      return [...recs, ...fls];
    });

    const triggerFileUpload = () => {
      fileInput.value?.click();
    };

    const handleFileUpload = (event) => {
      emit('upload-files', event);
    };

    const handleItemClick = (event, item) => {
      const itemId = item.id;
      
      // Ctrl/Cmd + Click: toggle selection
      if (event.ctrlKey || event.metaKey) {
        let newSelection = [...props.selectedFiles];
        if (newSelection.includes(itemId)) {
          newSelection = newSelection.filter(f => f !== itemId);
        } else {
          newSelection.push(itemId);
        }
        emit('update:selectedFiles', newSelection);
      }
      // Shift + Click: range selection
      else if (event.shiftKey && props.selectedFile) {
        const allIds = flatItems.value.map(i => i.id);
        const lastIndex = allIds.indexOf(props.selectedFile);
        const currentIndex = allIds.indexOf(itemId);
        
        if (lastIndex !== -1 && currentIndex !== -1) {
          const start = Math.min(lastIndex, currentIndex);
          const end = Math.max(lastIndex, currentIndex);
          const range = allIds.slice(start, end + 1);
          emit('update:selectedFiles', range);
        }
      }
      // Normal click: select single
      else {
        emit('update:selectedFiles', [itemId]);
        emit('update:selectedFile', itemId);
        if (item.type === 'recording') {
          emit('select-recording', itemId);
        } else {
          emit('select-file', item);
        }
      }
    };

    const handleItemContextMenu = (event, item) => {
      // If right-clicked file is not in selection, select it
      if (!props.selectedFiles.includes(item.id)) {
        emit('update:selectedFiles', [item.id]);
        emit('update:selectedFile', item.id);
      }
      
      contextMenu.value = {
        show: true,
        x: event.clientX,
        y: event.clientY,
        type: 'item',
        target: item.id
      };
    };

    const handleListContextMenu = (event) => {
      // No context menu for empty space if copy/paste is removed
      contextMenu.value = {
        show: false, // Ensure hidden or just don't set it
        x: 0,
        y: 0,
        type: null,
        target: null
      };
    };

    const closeContextMenu = () => {
      contextMenu.value.show = false;
    };

    const deleteFiles = () => {
      emit('delete-files');
      closeContextMenu();
    };
    
    const deleteSingleItem = (item) => {
      emit('update:selectedFiles', [item.id]);
      emit('update:selectedFile', item.id);
      setTimeout(() => {
        emit('delete-files');
      }, 0);
    };

    const exportSingleItem = (item) => {
      emit('update:selectedFiles', [item.id]);
      emit('update:selectedFile', item.id);
      setTimeout(() => {
        emit('export-files');
      }, 0);
    };
    
    const exportFile = () => {
      emit('export-files');
      closeContextMenu();
    };

    const renameFile = () => {
      emit('rename-file');
      closeContextMenu();
    };

    const handleDragOver = (event) => {
      isDraggingOver.value = true;
    };

    const handleDragLeave = (event) => {
      isDraggingOver.value = false;
    };

    const handleDrop = (event) => {
      isDraggingOver.value = false;
      emit('drop-files', event);
    };

    onMounted(() => {
      document.addEventListener('click', closeContextMenu);
    });

    onUnmounted(() => {
      document.removeEventListener('click', closeContextMenu);
    });

    return {
      fileInput,
      contextMenu,
      isDraggingOver,
      flatItems,
      triggerFileUpload,
      handleFileUpload,
      handleItemClick,
      handleItemContextMenu,
      handleListContextMenu,
      deleteFiles,
      deleteSingleItem,
      exportSingleItem,
      exportFile,
      renameFile,
      handleDragOver,
      handleDragLeave,
      handleDrop
    };
  }
};
</script>

<style scoped>
.file-explorer {
  height: 100%;
  background: #ffffff;
  display: flex;
  flex-direction: column;
  -webkit-user-select: none;
  user-select: none;
  cursor: default;
  /* Add transition for smooth width change if controlling width here, 
     but width is controlled by parent. 
     However, we might want to ensure content doesn't wrap weirdly. */
  overflow: hidden; 
  min-width: 0; /* Allow shrinking */
}

.file-explorer * {
  -webkit-user-select: none;
  user-select: none;
}

.explorer-header {
  display: flex;
  align-items: center;
  height: 44px; /* Hardcoded matching var(--header-height) */
  padding: 0 12px;
  background: #eef2e4;
  border-bottom: 1px solid #c1c989;
  gap: 6px;
  -webkit-app-region: drag;
  box-sizing: border-box;
}

.brand-logo {
  width: 20px;
  height: 20px;
  -webkit-app-region: no-drag;
  flex-shrink: 0;
}

.brand-text {
  font-family: system-ui, -apple-system, sans-serif;
  font-size: 14px;
  font-weight: 600;
  color: #4a5941;
  -webkit-app-region: no-drag;
  flex-shrink: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-right: auto;
}

.brand-by {
  font-weight: 400;
  color: #97ad8b;
  font-size: 11px;
}

.explorer-actions {
  display: flex;
  gap: 4px;
  -webkit-app-region: no-drag;
  flex-shrink: 0;
}

.icon-btn {
  background: transparent;
  border: none;
  color: #97ad8b;
  cursor: pointer;
  width: 28px;
  height: 28px;
  padding: 5px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  -webkit-app-region: no-drag;
}

.icon-btn:hover {
  background: #eef2e4;
  color: #5e6c53;
}

.back-btn {
  margin-right: 4px;
  flex-shrink: 0;
}

/* File List */
.file-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
}

.file-list.drag-over {
  background: rgba(193, 201, 137, 0.1);
}

.list-item {
  cursor: pointer;
  padding: 6px 16px;
  display: flex;
  align-items: center;
  transition: background 0.1s ease;
  height: 32px; /* Fixed height for consistency */
  box-sizing: border-box;
}

.list-item:hover {
  background: #f4f7ed;
}

.list-item.selected {
  background: #eef2e4;
}

.list-item.primary-selected {
  background: #e0e7a0;
  color: #3b4732;
}

.item-content {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  font-size: 13px;
  color: #5e6c53;
  overflow: hidden;
}

.file-icon {
  flex-shrink: 0;
  color: #9fb296;
}

.file-icon.recording {
  color: #81986a;
}

.folder-icon {
  color: #97ad8b;
  flex-shrink: 0;
}

.item-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.item-actions {
  display: flex;
  gap: 2px;
  opacity: 0;
  transition: opacity 0.2s;
}

.list-item:hover .item-actions {
  opacity: 1;
}

.action-btn {
  background: none;
  border: none;
  color: #9fb296;
  cursor: pointer;
  width: 24px;
  height: 24px;
  padding: 4px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  flex-shrink: 0;
}

.action-btn:hover {
  background: #eef2e4;
  color: #5e6c53;
}

.action-btn.delete {
  color: #c17a7a;
}

.action-btn.delete:hover {
  background: #fce8e8;
}

.empty-list {
  padding: 20px;
  text-align: center;
  color: #9fb296;
  font-size: 13px;
  font-style: italic;
  margin-top: 20px;
}

/* Context Menu */
.context-menu {
  position: fixed;
  background: #ffffff;
  border: 1px solid #e2e8d5;
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(94, 108, 83, 0.15);
  padding: 4px;
  z-index: 2000;
  min-width: 160px;
}

.context-menu-items {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.context-menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  font-size: 12px;
  color: #4a5941;
  cursor: pointer;
  border-radius: 4px;
  transition: background 0.1s;
}

.context-menu-item:hover:not(.disabled) {
  background: #eef2e4;
}

.context-menu-item.danger {
  color: #c17a7a;
}

.context-menu-item.danger:hover {
  background: #fce8e8;
}

.context-menu-item.disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.context-menu-item svg {
  flex-shrink: 0;
  opacity: 0.8;
}
</style>
