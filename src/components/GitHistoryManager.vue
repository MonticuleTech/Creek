<template>
  <div class="git-history-manager">
    <div v-if="commits.length > 0" class="timeline">
      <div v-for="(commit, index) in commits" :key="commit.hash" class="commit-item">
        <div class="commit-marker">
            <div class="dot"></div>
            <div class="line" v-if="index < commits.length - 1"></div>
        </div>
        
        <div class="commit-content">
            <div class="commit-message" :title="commit.message">{{ commit.message }}</div>
            <div class="commit-meta">
                <span class="commit-hash">{{ commit.hash.substring(0, 7) }}</span>
                <button 
                    v-if="index > 0" 
                    class="revert-btn" 
                    @click="rollback(commit.hash)"
                    title="Rollback to this version"
                >
                    Revert to this
                </button>
                <span v-else class="current-label">(Current)</span>
            </div>
        </div>
      </div>
    </div>
    
    <div v-else class="empty-state">
      <div class="empty-icon">Git</div>
      <p>No history available</p>
    </div>
    
    <div class="refresh-bar">
        <button class="refresh-btn" @click="fetchHistory">
            ⟳ Refresh History
        </button>
    </div>
  </div>
</template>

<script>
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { confirm } from '@tauri-apps/plugin-dialog';

export default {
  name: 'GitHistoryManager',
  setup() {
    const commits = ref([]);
    let updateUnlisten = null;

    const fetchHistory = async (retries = 3) => {
      try {
        const history = await invoke('get_git_history');
        if ((!history || history.length === 0) && retries > 0) {
            // Retry if empty (might be initializing)
            console.log(`Git history empty, retrying... (${retries} left)`);
            setTimeout(() => fetchHistory(retries - 1), 500);
            return;
        }
        commits.value = history || [];
      } catch (err) {
        console.error('Failed to fetch git history:', err);
        if (retries > 0) {
             setTimeout(() => fetchHistory(retries - 1), 1000);
        }
      }
    };

    const rollback = async (hash) => {
        const confirmed = await confirm(
            'Are you sure you want to rollback? Current changes will be lost.', 
            { title: 'Revert to Version', kind: 'warning' }
        );
        
        if (!confirmed) return;

        try {
            await invoke('rollback_to_commit', { commitHash: hash });
            // The backend will emit document-update, refreshing the view.
            // We should also refresh history as a new commit (restore) might be created?
            // Actually rollback usually checks out HEAD. 
            // In our implementation, we should check if rollback creates a new commit on top (revert style) or resets.
            // git_manager::rollback uses checkout_tree + set_head_detached.
            // So it moves HEAD. 
            setTimeout(fetchHistory, 500);
        } catch (err) {
            console.error('Rollback failed:', err);
            alert('Failed to rollback: ' + err);
        }
    };

    onMounted(async () => {
      fetchHistory();
      
      updateUnlisten = await listen('document-update', () => {
          // Refresh history when document updates (e.g. new commit)
          fetchHistory();
      });
    });
    
    onUnmounted(() => {
        if(updateUnlisten) updateUnlisten();
    })

    return {
      commits,
      fetchHistory,
      rollback
    };
  }
}
</script>

<style scoped>
.git-history-manager {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.timeline {
  flex: 1;
  overflow-y: auto;
  padding: 16px 12px;
  display: flex;
  flex-direction: column;
  gap: 0;
}

.commit-item {
  display: flex;
  gap: 12px;
  position: relative;
  padding-bottom: 24px;
}

.commit-item:last-child {
    padding-bottom: 0;
}

.commit-marker {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 16px;
    flex-shrink: 0;
}

.dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #ccc;
    border: 2px solid #fff;
    box-shadow: 0 0 0 1px #ccc;
    z-index: 1;
}

.commit-item:first-child .dot {
    background: #7d9569;
    box-shadow: 0 0 0 1px #7d9569;
}

.line {
    flex: 1;
    width: 1px;
    background: #e0e0e0;
    margin-top: 4px;
    min-height: 20px;
}

.commit-content {
    flex: 1;
    min-width: 0;
}

.commit-message {
    font-size: 14px;
    color: #333;
    font-weight: 500;
    margin-bottom: 4px;
    line-height: 1.4;
    word-break: break-word;
}

.commit-meta {
    display: flex;
    align-items: center;
    gap: 8px;
}

.commit-hash {
    font-family: monospace;
    font-size: 11px;
    color: #999;
    background: #f0f0f0;
    padding: 2px 4px;
    border-radius: 3px;
}

.revert-btn {
    font-size: 11px;
    padding: 2px 6px;
    border: 1px solid #ddd;
    background: #fff;
    border-radius: 4px;
    cursor: pointer;
    color: #666;
    transition: all 0.2s;
}

.revert-btn:hover {
    border-color: #888;
    color: #333;
}

.current-label {
    font-size: 11px;
    color: #7d9569; /* Green */
    font-weight: 600;
}

.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: #999;
  font-size: 14px;
}

.empty-icon {
    font-size: 18px;
    font-weight: bold;
    color: #eee;
    border: 2px solid #eee;
    border-radius: 4px;
    padding: 4px 8px;
    margin-bottom: 8px;
}

.refresh-bar {
  padding: 8px;
  border-top: 1px solid #eee;
  background: #f9f9f9;
  display: flex;
  justify-content: center;
}

.refresh-btn {
    background: none;
    border: none;
    font-size: 12px;
    color: #666;
    cursor: pointer;
    font-weight: 500;
}

.refresh-btn:hover {
    color: #333;
}
</style>
