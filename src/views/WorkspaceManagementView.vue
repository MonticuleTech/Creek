<template>
  <div class="workspace-management">
    <div class="management-header" data-tauri-drag-region>
      <button @click="navigateToWelcome" class="back-btn-header" title="Back to Welcome">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M19 12H5"/>
          <path d="M12 19l-7-7 7-7"/>
        </svg>
      </button>
      <div class="header-brand">
        <img src="@/assets/logo.svg" alt="Creek" class="header-logo" />
        <span class="header-title">Workspaces</span>
      </div>
    </div>
    
    <div class="management-content">
      <div class="workspaces-container">
        <div class="section-header">
          <h2>Your Workspaces</h2>
          <button class="create-button" @click="showCreateModal = true">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M12 5v14"/>
              <path d="M5 12h14"/>
            </svg>
            Create Workspace
          </button>
        </div>
        
        <div class="workspaces-list">
          <div 
            v-for="workspace in workspaces" 
            :key="workspace.id"
            class="workspace-card"
            @click="enterWorkspace(workspace.id)"
            @contextmenu.prevent="showContextMenu($event, workspace)"
          >
            <div class="workspace-icon">
              <svg width="32" height="32" viewBox="0 0 24 24" fill="currentColor">
                <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
              </svg>
            </div>
            <div class="workspace-info">
              <h3 class="workspace-name">{{ workspace.name }}</h3>
              <p class="workspace-meta">Created {{ formatDate(workspace.created_at) }}</p>
            </div>
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="workspace-arrow">
              <path d="M5 12h14"/>
              <path d="M12 5l7 7-7 7"/>
            </svg>
          </div>
          
          <div v-if="workspaces.length === 0" class="empty-state">
            <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
            </svg>
            <p>No workspaces yet</p>
            <button class="create-button-empty" @click="showCreateModal = true">
              Create Your First Workspace
            </button>
          </div>
        </div>
      </div>
    </div>
    
    <!-- Create Modal -->
    <div v-if="showCreateModal" class="modal-overlay" @click="showCreateModal = false">
      <div class="modal-content" @click.stop>
        <h3>Create New Workspace</h3>
        <input 
          v-model="newWorkspaceName" 
          type="text" 
          class="workspace-input" 
          placeholder="Workspace name"
          @keyup.enter="createWorkspace"
          ref="nameInput"
        />
        <div class="modal-actions">
          <button class="modal-button cancel" @click="showCreateModal = false">Cancel</button>
          <button class="modal-button create" @click="createWorkspace">Create</button>
        </div>
      </div>
    </div>
    
    <!-- Rename Modal -->
    <div v-if="showRenameModal" class="modal-overlay" @click="showRenameModal = false">
      <div class="modal-content" @click.stop>
        <h3>Rename Workspace</h3>
        <input 
          v-model="renameWorkspaceName" 
          type="text" 
          class="workspace-input" 
          placeholder="New workspace name"
          @keyup.enter="renameWorkspace"
          ref="renameInput"
        />
        <div class="modal-actions">
          <button class="modal-button cancel" @click="showRenameModal = false">Cancel</button>
          <button class="modal-button create" @click="renameWorkspace">Rename</button>
        </div>
      </div>
    </div>
    
    <!-- Context Menu -->
    <div 
      v-if="contextMenu.show" 
      class="context-menu"
      :style="{ top: contextMenu.y + 'px', left: contextMenu.x + 'px' }"
      @click.stop
    >
      <div class="context-menu-item" @click="handleRename">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>
          <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
        </svg>
        <span>Rename</span>
      </div>
      <div class="context-menu-item danger" @click="handleDelete">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="3 6 5 6 21 6"/>
          <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
        </svg>
        <span>Delete</span>
      </div>
    </div>
  </div>
</template>

<script>
import { ref, onMounted, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useRouter } from 'vue-router';
import { confirm as confirmDialog } from '@tauri-apps/plugin-dialog';

export default {
  name: 'WorkspaceManagementView',
  setup() {
    const router = useRouter();
    const workspaces = ref([]);
    const showCreateModal = ref(false);
    const showRenameModal = ref(false);
    const newWorkspaceName = ref('');
    const renameWorkspaceName = ref('');
    const nameInput = ref(null);
    const renameInput = ref(null);
    const selectedWorkspaceId = ref(null);
    
    const contextMenu = ref({
      show: false,
      x: 0,
      y: 0,
      workspace: null
    });
    
    const loadWorkspaces = async () => {
      try {
        const list = await invoke('list_workspaces');
        workspaces.value = list;
      } catch (err) {
        console.error('Failed to load workspaces:', err);
      }
    };
    
    const createWorkspace = async () => {
      if (!newWorkspaceName.value.trim()) {
        newWorkspaceName.value = '';
        return;
      }
      
      try {
        await invoke('create_workspace', { name: newWorkspaceName.value });
        showCreateModal.value = false;
        newWorkspaceName.value = '';
        await loadWorkspaces();
      } catch (err) {
        console.error('Failed to create workspace:', err);
        alert('Failed to create workspace: ' + err);
      }
    };
    
    const enterWorkspace = async (workspaceId) => {
      try {
        await invoke('set_current_workspace', { id: workspaceId });
        router.push(`/workspace/${workspaceId}`);
      } catch (err) {
        console.error('Failed to set current workspace:', err);
        alert('Failed to open workspace: ' + err);
      }
    };
    
    const formatDate = (timestamp) => {
      const date = new Date(timestamp);
      const now = new Date();
      const diff = now - date;
      const days = Math.floor(diff / (1000 * 60 * 60 * 24));
      
      if (days === 0) return 'Today';
      if (days === 1) return 'Yesterday';
      if (days < 7) return `${days} days ago`;
      if (days < 30) return `${Math.floor(days / 7)} weeks ago`;
      if (days < 365) return `${Math.floor(days / 30)} months ago`;
      return date.toLocaleDateString();
    };
    
    const showContextMenu = (event, workspace) => {
      contextMenu.value = {
        show: true,
        x: event.clientX,
        y: event.clientY,
        workspace
      };
    };
    
    const closeContextMenu = () => {
      contextMenu.value.show = false;
    };
    
    const handleRename = () => {
      selectedWorkspaceId.value = contextMenu.value.workspace.id;
      renameWorkspaceName.value = contextMenu.value.workspace.name;
      showRenameModal.value = true;
      closeContextMenu();
      nextTick(() => {
        renameInput.value?.focus();
        renameInput.value?.select();
      });
    };
    
    const renameWorkspace = async () => {
      if (!renameWorkspaceName.value.trim() || !selectedWorkspaceId.value) {
        return;
      }
      
      try {
        await invoke('rename_workspace', { 
          id: selectedWorkspaceId.value, 
          newName: renameWorkspaceName.value 
        });
        showRenameModal.value = false;
        renameWorkspaceName.value = '';
        selectedWorkspaceId.value = null;
        await loadWorkspaces();
      } catch (err) {
        console.error('Failed to rename workspace:', err);
        alert('Failed to rename workspace: ' + err);
      }
    };
    
    const handleDelete = async () => {
      const workspace = contextMenu.value.workspace;
      closeContextMenu();
      
      const confirmed = await confirmDialog(
        `Delete workspace "${workspace.name}"? This cannot be undone.`,
        { title: 'Delete Workspace', kind: 'warning' }
      );
      
      if (!confirmed) return;
      
      try {
        await invoke('delete_workspace', { id: workspace.id });
        await loadWorkspaces();
      } catch (err) {
        console.error('Failed to delete workspace:', err);
        alert('Failed to delete workspace: ' + err);
      }
    };
    
    onMounted(async () => {
      await loadWorkspaces();
      document.addEventListener('click', closeContextMenu);
      
      // Auto-focus input when modal opens
      nextTick(() => {
        if (showCreateModal.value) {
          nameInput.value?.focus();
        }
      });
    });
    
    const navigateToWelcome = () => {
      router.push('/');
    };
    
    return {
      workspaces,
      showCreateModal,
      showRenameModal,
      newWorkspaceName,
      renameWorkspaceName,
      nameInput,
      renameInput,
      contextMenu,
      createWorkspace,
      renameWorkspace,
      enterWorkspace,
      formatDate,
      showContextMenu,
      handleRename,
      handleDelete,
      navigateToWelcome
    };
  }
};
</script>

<style scoped>
.workspace-management {
  width: 100%;
  height: 100vh;
  background: #f9faf7;
  display: flex;
  flex-direction: column;
  -webkit-user-select: none;
  user-select: none;
}

.management-header {
  height: 44px;
  background: #eef2e4;
  border-bottom: 1px solid #c1c989;
  display: flex;
  align-items: center;
  padding: 0 20px;
  -webkit-app-region: drag;
}

.header-brand {
  display: flex;
  align-items: center;
  gap: 10px;
}

.header-logo {
  width: 24px;
  height: 24px;
}

.header-title {
  font-family: system-ui, -apple-system, sans-serif;
  font-size: 16px;
  font-weight: 600;
  color: #4a5941;
}

.back-btn-header {
  background: transparent;
  border: none;
  color: #97ad8b;
  cursor: pointer;
  width: 32px;
  height: 32px;
  padding: 6px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  -webkit-app-region: no-drag;
  margin-right: 12px;
}

.back-btn-header:hover {
  background: #ffffff;
  color: #5e6c53;
}

.management-content {
  flex: 1;
  overflow-y: auto;
  padding: 40px;
}

.workspaces-container {
  max-width: 800px;
  margin: 0 auto;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.section-header h2 {
  font-family: system-ui, -apple-system, sans-serif;
  font-size: 24px;
  font-weight: 600;
  color: #4a5941;
  margin: 0;
}

.create-button {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 20px;
  background: #5e6c53;
  color: #ffffff;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
}

.create-button:hover {
  background: #4a5941;
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(94, 108, 83, 0.2);
}

.workspaces-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.workspace-card {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 20px;
  background: #ffffff;
  border: 1px solid #e2e8d5;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.workspace-card:hover {
  background: #f4f7ed;
  border-color: #c1c989;
  transform: translateX(4px);
  box-shadow: 0 4px 12px rgba(94, 108, 83, 0.1);
}

.workspace-icon {
  flex-shrink: 0;
  width: 48px;
  height: 48px;
  background: #eef2e4;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #81986a;
}

.workspace-info {
  flex: 1;
  min-width: 0;
}

.workspace-name {
  font-family: system-ui, -apple-system, sans-serif;
  font-size: 16px;
  font-weight: 600;
  color: #4a5941;
  margin: 0 0 4px 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.workspace-meta {
  font-family: system-ui, -apple-system, sans-serif;
  font-size: 13px;
  color: #97ad8b;
  margin: 0;
}

.workspace-arrow {
  flex-shrink: 0;
  color: #97ad8b;
  transition: all 0.2s ease;
}

.workspace-card:hover .workspace-arrow {
  color: #5e6c53;
  transform: translateX(4px);
}

.empty-state {
  text-align: center;
  padding: 60px 20px;
  color: #97ad8b;
}

.empty-state svg {
  margin-bottom: 16px;
  opacity: 0.5;
}

.empty-state p {
  font-size: 16px;
  margin-bottom: 24px;
}

.create-button-empty {
  padding: 12px 24px;
  background: #5e6c53;
  color: #ffffff;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
}

.create-button-empty:hover {
  background: #4a5941;
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(94, 108, 83, 0.2);
}

.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(74, 89, 65, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  backdrop-filter: blur(4px);
}

.modal-content {
  background: #ffffff;
  border-radius: 12px;
  padding: 32px;
  width: 90%;
  max-width: 400px;
  box-shadow: 0 8px 32px rgba(94, 108, 83, 0.2);
}

.modal-content h3 {
  font-family: system-ui, -apple-system, sans-serif;
  font-size: 20px;
  font-weight: 600;
  color: #4a5941;
  margin: 0 0 20px 0;
}

.workspace-input {
  width: 100%;
  padding: 12px 16px;
  border: 1px solid #e2e8d5;
  border-radius: 6px;
  font-size: 14px;
  color: #4a5941;
  transition: all 0.2s ease;
  box-sizing: border-box;
}

.workspace-input:focus {
  outline: none;
  border-color: #81986a;
  box-shadow: 0 0 0 3px rgba(129, 152, 106, 0.1);
}

.modal-actions {
  display: flex;
  gap: 12px;
  margin-top: 20px;
  justify-content: flex-end;
}

.modal-button {
  padding: 10px 20px;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
}

.modal-button.cancel {
  background: #f4f7ed;
  color: #5e6c53;
}

.modal-button.cancel:hover {
  background: #eef2e4;
}

.modal-button.create {
  background: #5e6c53;
  color: #ffffff;
}

.modal-button.create:hover {
  background: #4a5941;
}

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

.context-menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  font-size: 13px;
  color: #4a5941;
  cursor: pointer;
  border-radius: 4px;
  transition: background 0.1s;
}

.context-menu-item:hover {
  background: #eef2e4;
}

.context-menu-item.danger {
  color: #c17a7a;
}

.context-menu-item.danger:hover {
  background: #fce8e8;
}

.context-menu-item svg {
  flex-shrink: 0;
  opacity: 0.8;
}
</style>
