<template>
  <div class="panel-wrapper" :class="{ 'is-collapsed': isCollapsed }">
    <!-- Animated Container -->
    <div class="collapsible-panel">
        
      <!-- Header / Toggle Area -->
      <div class="panel-header" @click="handleHeaderClick">
        <!-- Collapsed: Centered Icon -->
        <div class="collapsed-view" v-if="isCollapsed">
           <ClipboardCheckIcon class="fab-icon" />
        </div>

        <!-- Expanded: Tabs and Minimize -->
        <div class="expanded-view" v-else>
          <div class="tabs">
            <button 
              class="tab-btn" 
              :class="{ active: activeTab === 'todos' }" 
              @click.stop="activeTab = 'todos'"
            >
              To-do
            </button>
            <button 
              class="tab-btn" 
              :class="{ active: activeTab === 'history' }" 
              @click.stop="activeTab = 'history'"
            >
              History
            </button>
          </div>
          
          <button class="minimize-btn" @click.stop="toggleCollapse" title="Minimize">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="6 9 12 15 18 9"></polyline>
            </svg>
          </button>
        </div>
      </div>

      <!-- Content Area (Only rendered/visible when expanded) -->
      <div class="panel-content" v-show="!isCollapsed">
        <transition name="fade" mode="out-in">
            <div v-if="activeTab === 'todos'" key="todos" class="tab-content">
              <TodoManager />
            </div>
            <div v-else key="history" class="tab-content">
              <GitHistoryManager />
            </div>
        </transition>
      </div>
      
    </div>
  </div>
</template>

<script>
import { ref } from 'vue';
import TodoManager from './TodoManager.vue';
import GitHistoryManager from './GitHistoryManager.vue';
import ClipboardCheckIcon from './ClipboardCheckIcon.vue';

export default {
  name: 'CollapsiblePanel',
  components: {
    TodoManager,
    GitHistoryManager,
    ClipboardCheckIcon
  },
  setup() {
    const isCollapsed = ref(true); 
    const activeTab = ref('todos');

    const toggleCollapse = () => {
      isCollapsed.value = !isCollapsed.value;
    };

    const handleHeaderClick = () => {
        if (isCollapsed.value) {
            isCollapsed.value = false;
        }
    }

    return {
      isCollapsed,
      activeTab,
      toggleCollapse,
      handleHeaderClick
    };
  }
}
</script>

<style scoped>
/* Wrapper for positioning */
.panel-wrapper {
  position: fixed;
  bottom: 24px;
  right: 24px;
  z-index: 1000;
  display: flex;
  flex-direction: column;
  align-items: flex-end; /* Align right so it expands leftwards/upwards intuitively */
}

/* The Morphing Box */
.collapsible-panel {
  background: #ffffff;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
  overflow: hidden;
  transition: all 0.5s cubic-bezier(0.19, 1, 0.22, 1); /* Elegant easing */
  
  /* Expanded State Defaults */
  width: 380px;
  height: 500px;
  border-radius: 12px;
  border: 1px solid #e0e0e0;
}

/* Collapsed State Overrides */
.panel-wrapper.is-collapsed .collapsible-panel {
  width: 56px;
  height: 56px;
  border-radius: 50%; /* Circle */
  border: none;
  background: #7d9569; /* Theme Green */
  box-shadow: 0 8px 16px rgba(125, 149, 105, 0.4);
  cursor: pointer;
}

.panel-header {
  height: 48px; /* Standard header height */
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}

/* Collapsed Header: Centered Icon */
.panel-wrapper.is-collapsed .panel-header {
    height: 100%;
    justify-content: center;
    padding: 0;
}

.collapsed-view {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    color: white;
}

.fab-icon {
    width: 28px;
    height: 28px;
    /* Optional: Animate icon on hover */
    transition: transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.panel-wrapper.is-collapsed .collapsible-panel:hover {
    transform: scale(1.05);
}

/* Expanded Header: Tabs and Minimize */
.expanded-view {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 0 4px;
    height: 100%;
    background: #f8f9fa;
    border-bottom: 1px solid #e0e0e0;
}

.tabs {
  display: flex;
  gap: 0;
  height: 100%;
}

.tab-btn {
  padding: 0 20px;
  border: none;
  background: transparent;
  color: #666;
  font-weight: 500;
  font-size: 13px;
  cursor: pointer;
  height: 100%;
  position: relative;
  transition: all 0.2s;
}

.tab-btn:hover {
  color: #333;
  background: rgba(0,0,0,0.02);
}

.tab-btn.active {
  color: #7d9569;
  font-weight: 600;
}

.tab-btn.active::after {
    content: '';
    position: absolute;
    bottom: 0px;
    left: 0;
    right: 0;
    height: 2px;
    background: #7d9569;
}

.minimize-btn {
  background: none;
  border: none;
  cursor: pointer;
  padding: 8px 12px;
  color: #888;
  display: flex;
  align-items: center;
  justify-content: center;
}

.minimize-btn:hover {
    color: #333;
}

.panel-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  height: calc(100% - 48px);
}

.tab-content {
    height: 100%;
    overflow: hidden;
}

/* Transitions */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
