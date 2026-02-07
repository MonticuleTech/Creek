<template>
  <div class="workspace-view">
    <Toast />
    <ThinkingToast :visible="isThinking" />
    <CollapsiblePanel />
    <RenameModal
      :is-open="isRenameModalOpen"
      :current-name="renameTargetName"
      @close="isRenameModalOpen = false"
      @confirm="handleRenameConfirm"
    />
    <div class="workspace-content">
      <!-- Left: IDE-style file tree (Sidebar) -->
      <WorkspaceSidebar
        :files="files"
        :recordings="recordings"
        :selected-files="selectedFiles"
        :selected-file="selectedFile"
        :style="{ 
          width: isSidebarCollapsed ? '0px' : leftPanelWidth + 'px',
          transition: 'width 0.3s ease-in-out'
        }"
        :class="{ collapsed: isSidebarCollapsed }"
        @update:selected-files="selectedFiles = $event"
        @update:selected-file="selectedFile = $event"
        @select-file="(file) => selectFile(file)"
        @select-recording="(rec) => selectRecording(rec)"
        @refresh="refreshData"
        @upload-files="handleFileUpload"
        @delete-files="deleteFiles"
        @rename-file="renameFile"
        @export-files="exportFile"
        @drop-files="handleDrop"
        @navigate-back="navigateBack"
      />

      <!-- Resizer with Toggle Button -->
      <div class="resizer" @mousedown="startResize">
        <button 
          class="sidebar-toggle-btn"
          @mousedown.stop 
          @click="toggleSidebar"
          :title="isSidebarCollapsed ? 'Expand Sidebar' : 'Collapse Sidebar'"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
             <path v-if="!isSidebarCollapsed" d="M15 18l-6-6 6-6"/>
             <path v-else d="M9 18l6-6-6-6"/>
          </svg>
        </button>
      </div>

      <!-- Right: Recording and document editing -->
      <div class="main-panel">
        <!-- Toolbar -->
        <WorkspaceToolbar
          :is-recording="isRecording"
          :is-paused="isPaused"
          :mic-volume="micVolume"
          @start-recording="startRecording"
          @stop-recording="stopRecording"
          @toggle-pause="togglePause"
          @save-document="saveCurrentDocument"
          @reset-document="resetDocument"
          @new-document="createNewDocument"
        />

        <!-- Live Canvas (Markdown viewer + editor) -->
        <div class="canvas-area">
          <LiveCanvas 
            :content="documentContent" 
            :doc-id="currentEditingFile"
            :is-recording="isRecording"
            @update:content="handleDocumentUpdate"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script>
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useRouter } from 'vue-router';
import { 
  BaseDirectory, 
  exists, 
  mkdir, 
  readDir, 
  readTextFile, 
  writeTextFile,
  remove,
  rename,
  stat
} from '@tauri-apps/plugin-fs';
import { confirm as confirmDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
import Toast from '@/components/Toast.vue';
import ThinkingToast from '@/components/ThinkingToast.vue';
import CollapsiblePanel from '@/components/CollapsiblePanel.vue';
import LiveCanvas from '@/components/LiveCanvas.vue';
import RenameModal from '@/components/RenameModal.vue';
import WorkspaceSidebar from '@/components/workspace/WorkspaceSidebar.vue';
import WorkspaceToolbar from '@/components/workspace/WorkspaceToolbar.vue';

export default {
  name: 'WorkspaceView',
  components: {
    Toast,
    ThinkingToast,
    CollapsiblePanel,
    LiveCanvas,
    RenameModal,
    WorkspaceSidebar,
    WorkspaceToolbar
  },
  setup() {
    const router = useRouter();
    const files = ref([]);
    const recordings = ref([]);
    const isRecording = ref(false);
    const isPaused = ref(false);
    const documentContent = ref('');
    const micVolume = ref(0);
    const isThinking = ref(false); // Reactive state for agent thinking
    
    // Selection state
    const selectedFile = ref(null);
    const selectedFiles = ref([]);
    const currentEditingFile = ref(null);
    const isEditingRecording = ref(false);

    // Panel resize
    const leftPanelWidth = ref(285);
    const isResizing = ref(false);
    const isSidebarCollapsed = ref(false);

    const toggleSidebar = () => {
      isSidebarCollapsed.value = !isSidebarCollapsed.value;
    };

    // Local storage
    const localFiles = ref([]);
    const localRecordings = ref({});
    
    // Rename Modal
    const isRenameModalOpen = ref(false);
    const renameTargetName = ref('');

    let documentUpdateUnlisten = null;
    let recordingStartedUnlisten = null;
    let saveTimer = null;
    let pendingSave = null;
    let audioContext = null;
    let analyser = null;
    let microphone = null;

    // 初始化文件系统目录
    const initFileSystem = async () => {
      try {
        const creekDirExists = await exists('creek', { baseDir: BaseDirectory.AppData });
        if (!creekDirExists) {
          await mkdir('creek', { baseDir: BaseDirectory.AppData, recursive: true });
        }

        const uploadsDirExists = await exists('creek/uploads', { baseDir: BaseDirectory.AppData });
        if (!uploadsDirExists) {
          await mkdir('creek/uploads', { baseDir: BaseDirectory.AppData, recursive: true });
        }
      } catch (err) {
        console.error('Failed to initialize file system:', err);
      }
    };

    const loadFiles = async () => {
      try {
        const entries = await readDir('creek/uploads', { baseDir: BaseDirectory.AppData });
        const loadedFiles = [];
        
        for (const entry of entries) {
          if (entry.isDirectory) {
            loadedFiles.push({
              name: entry.name,
              content: null,
              isDirectory: true,
              uploadedAt: Date.now()
            });
          } else if (entry.name && (entry.name.endsWith('.txt') || entry.name.endsWith('.md'))) {
            try {
              const filePath = `creek/uploads/${entry.name}`;
              const content = await readTextFile(filePath, { baseDir: BaseDirectory.AppData });
              const fileStat = await stat(filePath, { baseDir: BaseDirectory.AppData });
              
              const time = fileStat.mtime ? new Date(fileStat.mtime).getTime() : Date.now();

              loadedFiles.push({
                name: entry.name,
                content: content,
                isDirectory: false,
                uploadedAt: time
              });
            } catch (err) {
              console.error(`Failed to read file ${entry.name}:`, err);
            }
          }
        }
        
        localFiles.value = loadedFiles;
        files.value = loadedFiles;
        console.log(`Loaded ${loadedFiles.length} uploaded files`);
      } catch (err) {
        console.error('Failed to load file list:', err);
        localFiles.value = [];
        files.value = [];
      }
    };

    const loadRecordings = async () => {
      try {
        const recordingsList = await invoke('list_recordings');
        const recordingsMap = {};
        
        for (const recording of recordingsList) {
          recordingsMap[recording.id] = recording.content;
          // Ensure recording object has created_at (backend sends it now)
        }
        
        localRecordings.value = recordingsMap;
        recordings.value = recordingsList;
        console.log(`Loaded ${recordingsList.length} recordings from backend`);
      } catch (err) {
        console.error('Failed to load recordings:', err);
        localRecordings.value = {};
        recordings.value = [];
      }
    };

    const refreshData = () => {
      Promise.all([loadFiles(), loadRecordings()]);
    };

    const handleFileUpload = async (event) => {
      const selectedFiles = event.target.files;
      if (!selectedFiles || selectedFiles.length === 0) return;

      for (let file of selectedFiles) {
        try {
          const reader = new FileReader();
          reader.onload = async (e) => {
            const content = e.target.result;
            const fileName = file.name.endsWith('.txt') ? file.name : `${file.name}.txt`;
            
            await writeTextFile(`creek/uploads/${fileName}`, content, { 
              baseDir: BaseDirectory.AppData 
            });
            
            const newFile = {
              name: fileName,
              content: content,
              uploadedAt: Date.now()
            };
            localFiles.value.push(newFile);
            files.value = [...localFiles.value];
            console.log(`File saved: ${fileName}`);
          };
          reader.readAsText(file);
        } catch (err) {
          console.error('Failed to upload file:', err);
        }
      }
      if (event.target) {
        event.target.value = '';
      }
    };

    const selectFile = async (file) => {
      await flushPendingSaveIfNeeded();
      selectedFile.value = file.name;
      selectedFiles.value = [file.name];
      currentEditingFile.value = file.name;
      isEditingRecording.value = false;
      documentContent.value = file.content || '';
    };

    const selectRecording = async (recordingName) => {
      await flushPendingSaveIfNeeded();
      selectedFile.value = recordingName;
      selectedFiles.value = [recordingName];
      currentEditingFile.value = recordingName;
      isEditingRecording.value = true;
      documentContent.value = localRecordings.value[recordingName] || '';
      
      try {
        await invoke('load_recording', { recordingId: recordingName });
      } catch (err) {
        console.error('Failed to sync backend recording:', err);
      }
    };

    const createNewDocument = async () => {
      const customName = prompt('Enter recording name (optional, default: "New Recording"):');
      
      try {
        await flushPendingSaveIfNeeded();
        // Send empty string if null, backend handles default name
        const finalName = customName === null ? '' : customName;
        
        // Backend now returns full RecordingInfo object
        const newRecording = await invoke('create_recording', { 
          name: finalName 
        });
        
        const recordingId = newRecording.id;
        const displayName = newRecording.name;
        
        // Add to local content map
        localRecordings.value[recordingId] = '';
        
        // Add to list
        recordings.value = [{
            id: recordingId,
            name: displayName,
            content: '',
            path: newRecording.path,
            has_git: false
        }, ...recordings.value];
        
        selectedFile.value = recordingId;
        selectedFiles.value = [recordingId];
        currentEditingFile.value = recordingId;
        isEditingRecording.value = true;
        documentContent.value = '';
        
        console.log(`Created new recording: ${recordingId} ("${displayName}")`);
      } catch (err) {
        console.error('Failed to create new recording:', err);
        alert('Failed to create recording: ' + err);
      }
    };

    const deleteFiles = async () => {
      if (selectedFiles.value.length === 0) return;

      const filesToDelete = [...selectedFiles.value];
      const isRecordingFiles = filesToDelete.map(f => recordings.value.some(r => r.id === f));
      
      const displayNames = filesToDelete.map((f, i) => {
        if (isRecordingFiles[i]) {
            const rec = recordings.value.find(r => r.id === f);
            return rec ? rec.name : f; // Fallback to ID if not found
        }
        return f; // Local files use name as ID
      });

      const message = displayNames.length === 1
        ? `Delete "${displayNames[0]}"?`
        : `Delete ${displayNames.length} files?`;

      const confirmed = await confirmDialog(message, { title: 'Creek', kind: 'warning' });
      if (!confirmed) return;

      console.log('Deleting files:', filesToDelete);

      for (let i = 0; i < filesToDelete.length; i++) {
        const fileName = filesToDelete[i];
        const isRecordingFile = isRecordingFiles[i];
        try {
          console.log(`Deleting ${fileName}, isRecording: ${isRecordingFile}`);

          if (isRecordingFile) {
            await invoke('delete_recording', { recordingId: fileName });
            delete localRecordings.value[fileName];
            recordings.value = recordings.value.filter(r => r.id !== fileName);
          } else {
            await remove(`creek/uploads/${fileName}`, { baseDir: BaseDirectory.AppData });
            localFiles.value = localFiles.value.filter(f => f.name !== fileName);
            files.value = [...localFiles.value];
          }

          if (currentEditingFile.value === fileName) {
            selectedFile.value = null;
            selectedFiles.value = [];
            currentEditingFile.value = null;
            isEditingRecording.value = false;
            documentContent.value = '';
          }

          console.log(`Deleted: ${fileName}`);
        } catch (err) {
          console.error(`Failed to delete ${fileName}:`, err);
          alert(`Failed to delete ${fileName}: ${err}`);
        }
      }

      selectedFiles.value = [];
    };

    // Drop Files
    const handleDrop = async (event) => {
      const droppedFiles = event.dataTransfer.files;
      if (!droppedFiles || droppedFiles.length === 0) return;

      for (let file of droppedFiles) {
        try {
          const reader = new FileReader();
          reader.onload = async (e) => {
            const content = e.target.result;
            const fileName = file.name.match(/\.(md|txt)$/) ? file.name : `${file.name}.txt`;

            await writeTextFile(`creek/uploads/${fileName}`, content, {
              baseDir: BaseDirectory.AppData
            });

            const newFile = {
              name: fileName,
              content: content,
              uploadedAt: Date.now()
            };
            localFiles.value.push(newFile);
            files.value = [...localFiles.value];
            console.log(`Dropped file saved: ${fileName}`);
          };
          reader.readAsText(file);
        } catch (err) {
          console.error('Failed to handle dropped file:', err);
        }
      }
    };


    const renameFile = () => {
      if (!selectedFile.value) {
        console.warn('No file selected to rename');
        return;
      }
      
      const fileId = selectedFile.value;
      
      // Determine current display name
      let currentName = fileId;
      const rec = recordings.value.find(r => r.id === fileId);
      if (rec) {
        currentName = rec.name;
      } else {
        // Assume file
        currentName = fileId; // Files use name as ID
      }
      
      renameTargetName.value = currentName;
      isRenameModalOpen.value = true;
    };

    const handleRenameConfirm = async (newName) => {
      if (!newName || newName === renameTargetName.value) return;
      
      const oldId = selectedFile.value; // selectedFile is the ID for recordings
      const isRecording = recordings.value.some(r => r.id === oldId);

      try {
        if (isRecording) {
            // Call backend: old_id is the ID, new_id is the new NAME
          await invoke('rename_recording', { oldId: oldId, newId: newName });
          
          // ID does NOT change. Only name changes.
          // Update recordings list metadata
          const idx = recordings.value.findIndex(r => r.id === oldId);
          if (idx !== -1) {
              const updated = { ...recordings.value[idx], name: newName };
              // Force reactivity update
              const list = [...recordings.value];
              list[idx] = updated;
              recordings.value = list;
          }
           // No need to update localRecordings keys or selectedFile, as ID is constant! 
          
        } else {
            // Files logic (name IS id)
            // Need to use renameTargetName.value because selectedFile might have changed? No, it's modal.
            // But for files, renameTargetName was initialized with name.
            let oldName = renameTargetName.value; 
            // Better to use selectedFile if it hasn't changed. 
            // Actually, for files, ID=Name. So oldId above is correct.
             let oldNameFile = oldId;
             
          try {
            await rename(
              `creek/uploads/${oldNameFile}`,
              `creek/uploads/${newName}`,
              { baseDir: BaseDirectory.AppData }
            );
            
            // Update local files list
            const fileIdx = localFiles.value.findIndex(f => f.name === oldNameFile);
            if (fileIdx !== -1) {
              localFiles.value[fileIdx].name = newName;
            }
            files.value = [...localFiles.value];
            
            // For files, ID changes!
            selectedFile.value = newName;
            selectedFiles.value = [newName];
            if (currentEditingFile.value === oldNameFile) {
              currentEditingFile.value = newName;
            }
            
          } catch(e) {
             console.error('FS Rename failed', e);
             alert('Rename failed: ' + e);
             return;
          }
        }

        console.log(`Renamed/Updated ${oldId} to ${newName}`);
      } catch (err) {
        console.error('Failed to rename:', err);
        alert('Failed to rename: ' + err);
      }
    };

    const exportFile = async () => {
      if (!selectedFile.value) return;
      const fileId = selectedFile.value;
      const isRecording = recordings.value.some(r => r.id === fileId);
      
      let content = '';
      let displayFileName = fileId;

      if (isRecording) {
        content = localRecordings.value[fileId] || '';
        const rec = recordings.value.find(r => r.id === fileId);
        if (rec) displayFileName = rec.name;
      } else {
        const file = localFiles.value.find(f => f.name === fileId);
        content = file ? file.content : '';
        displayFileName = fileId;
      }
      
      // Default to markdown extension
      const defaultName = displayFileName.replace(/\.[^/.]+$/, "") + ".md";

      try {
        // Open system Save Dialog
        const path = await saveDialog({
          defaultPath: defaultName,
          filters: [
            {
              name: 'Markdown',
              extensions: ['md']
            },
            {
              name: 'Text Document',
              extensions: ['txt']
            },
            {
              name: 'All Files',
              extensions: ['*']
            }
          ]
        });
        
        if (path) {
          await writeTextFile(path, content);
          console.log(`Exported ${displayFileName} to ${path}`);
        }
      } catch (err) {
        console.error('Failed to export file:', err);
        alert('Export failed: ' + err);
      }
    };

    const startMicMonitoring = async () => {
      try {
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        audioContext = new (window.AudioContext || window.webkitAudioContext)();
        analyser = audioContext.createAnalyser();
        microphone = audioContext.createMediaStreamSource(stream);
        analyser.fftSize = 2048;
        analyser.smoothingTimeConstant = 0;
        microphone.connect(analyser);
        
        const bufferLength = analyser.fftSize;
        const dataArray = new Uint8Array(bufferLength);
        
        const updateVolume = () => {
          if (!isRecording.value || isPaused.value) {
            micVolume.value = 0;
            requestAnimationFrame(updateVolume);
            return;
          }
          
          analyser.getByteTimeDomainData(dataArray);
          
          let max = 0;
          for (let i = 0; i < bufferLength; i++) {
            const amplitude = Math.abs(dataArray[i] - 128);
            if (amplitude > max) {
              max = amplitude;
            }
          }
          
          micVolume.value = Math.min(100, (max / 128) * 100);
          requestAnimationFrame(updateVolume);
        };
        
        requestAnimationFrame(updateVolume);
      } catch (err) {
        console.error('Failed to access microphone:', err);
      }
    };

    const stopMicMonitoring = () => {
      if (microphone) {
        const stream = microphone.mediaStream;
        if (stream) {
          stream.getTracks().forEach(track => track.stop());
        }
        microphone.disconnect();
        microphone = null;
      }
      if (audioContext) {
        audioContext.close();
        audioContext = null;
      }
      analyser = null;
      micVolume.value = 0;
    };

    const startRecording = async () => {
      try {
        if (!currentEditingFile.value || !isEditingRecording.value) {
          console.log('No recording file selected, creating new one');
          await createNewDocument();
          await new Promise(resolve => setTimeout(resolve, 200));
        }
        
        const recordingId = currentEditingFile.value;
        console.log(`Recording to file: ${recordingId}`);
        
        await invoke('start_recording', { recordingId: recordingId });
        isRecording.value = true;
        isPaused.value = false;
        
        await startMicMonitoring();
      } catch (err) {
        console.error('Failed to start recording:', err);
      }
    };

    const togglePause = async () => {
      try {
        if (isPaused.value) {
          await invoke('resume_recording');
          isPaused.value = false;
        } else {
          await invoke('pause_recording');
          isPaused.value = true;
        }
      } catch (err) {
        console.error('Failed to toggle pause:', err);
      }
    };

    const stopRecording = async () => {
      try {
        await invoke('stop_recording');
        isRecording.value = false;
        isPaused.value = false;
        
        stopMicMonitoring();
        
        await loadRecordings();
      } catch (err) {
        console.error('Failed to stop recording:', err);
      }
    };

    const resetDocument = async () => {
      try {
        const confirmed = await confirmDialog('Are you sure you want to reset the document? This will clear all content.', { 
            title: 'Reset Document',
            kind: 'warning'
        });
        
        if (!confirmed) return;

        await invoke('reset_document');
        
        // Save empty state to disk immediately
        if (currentEditingFile.value) {
            await persistDocument(currentEditingFile.value, '', isEditingRecording.value);
        }
        
        documentContent.value = '';
        // Keep the file open so user can continue editing the empty document
        // currentEditingFile.value = null;
        // selectedFile.value = null;
        
        console.log('Document reset and saved as empty.');
      } catch (err) {
        console.error('Failed to reset document:', err);
      }
    };

    const persistDocument = async (fileId, content, isRecordingFile) => {
      if (!fileId) return;
      if (isRecordingFile) {
        await invoke('update_recording', { recordingId: fileId, content });
        localRecordings.value[fileId] = content;
        console.log(`Saved recording: ${fileId}`);
        return;
      }

      await writeTextFile(`creek/uploads/${fileId}`, content, {
        baseDir: BaseDirectory.AppData
      });
      const fileIndex = localFiles.value.findIndex(f => f.name === fileId);
      if (fileIndex !== -1) {
        localFiles.value[fileIndex].content = content;
        files.value = [...localFiles.value];
      }
      console.log(`Saved file: ${fileId}`);
    };

    const scheduleAutoSave = (fileId, content, isRecordingFile) => {
      pendingSave = { fileId, content, isRecordingFile };
      if (saveTimer) {
        clearTimeout(saveTimer);
      }
      saveTimer = setTimeout(async () => {
        const snapshot = pendingSave;
        pendingSave = null;
        saveTimer = null;
        if (!snapshot) return;
        try {
          await persistDocument(snapshot.fileId, snapshot.content, snapshot.isRecordingFile);
        } catch (err) {
          console.error('Failed to auto-save:', err);
        }
      }, 500);
    };

    const flushPendingSave = async () => {
      if (saveTimer) {
        clearTimeout(saveTimer);
        saveTimer = null;
      }
      if (!pendingSave) return;
      const snapshot = pendingSave;
      pendingSave = null;
      try {
        await persistDocument(snapshot.fileId, snapshot.content, snapshot.isRecordingFile);
      } catch (err) {
        console.error('Failed to flush save:', err);
      }
    };

    const flushPendingSaveIfNeeded = async () => {
      if (isRecording.value) return;
      await flushPendingSave();
    };

    const handleDocumentUpdate = (newContent) => {
      documentContent.value = newContent;
      
      if (currentEditingFile.value) {
        invoke('update_document', { content: newContent }).catch(err => {
          console.error('[Frontend] Failed to sync document with pipeline:', err);
        });

        if (!isRecording.value) {
          const fileId = currentEditingFile.value;
          const isRecordingFile = isEditingRecording.value;
          scheduleAutoSave(fileId, newContent, isRecordingFile);
        }
      }
    };

    const saveCurrentDocument = async () => {
      if (!documentContent.value.trim()) return;

      try {
        if (!currentEditingFile.value) {
          console.log('No file selected to save');
          return;
        }

        if (saveTimer) {
          clearTimeout(saveTimer);
          saveTimer = null;
        }
        pendingSave = null;
        await persistDocument(currentEditingFile.value, documentContent.value, isEditingRecording.value);
      } catch (err) {
        console.error('Failed to save document:', err);
      }
    };

    const startResize = (e) => {
      isResizing.value = true;
      document.addEventListener('mousemove', handleResize);
      document.addEventListener('mouseup', stopResize);
      e.preventDefault();
    };

    const handleResize = (e) => {
      if (!isResizing.value) return;
      const newWidth = e.clientX;
      if (newWidth >= 285 && newWidth <= 500) {
        leftPanelWidth.value = newWidth;
      }
    };

    const stopResize = () => {
      isResizing.value = false;
      document.removeEventListener('mousemove', handleResize);
      document.removeEventListener('mouseup', stopResize);
    };

    const navigateBack = () => {
      router.push('/workspaces');
    };

    onMounted(async () => {
      await initFileSystem();
      await Promise.all([loadFiles(), loadRecordings()]);
      
      // Auto-open newest file
      const allItems = [];
      
      // Add files
      files.value.forEach(f => {
          allItems.push({
              id: f.name,
              data: f,
              type: 'file',
              time: f.uploadedAt || 0
          });
      });
      
      // Add recordings
      recordings.value.forEach(r => {
          allItems.push({
              id: r.id,
              data: r,
              type: 'recording',
              time: r.created_at || 0 // Use backend timestamp
          });
      });
      
      // Sort Descending
      allItems.sort((a, b) => b.time - a.time);
      
      if (allItems.length > 0) {
          const newest = allItems[0];
          console.log('Auto-opening newest item:', newest.id, new Date(newest.time).toLocaleString());
          if (newest.type === 'recording') {
              selectRecording(newest.id);
          } else {
              selectFile(newest.data);
          }
      }
      
      documentUpdateUnlisten = await listen('document-update', (event) => {
        console.log('[Frontend] Received document-update:', event.payload);
        documentContent.value = event.payload.content;
        
        if (isRecording.value && currentEditingFile.value && isEditingRecording.value) {
          localRecordings.value[currentEditingFile.value] = event.payload.content;
        }
      });

      const agentStatusUnlisten = await listen('agent-status', (event) => {
          if (event.payload.status === 'thinking') {
              isThinking.value = true;
          } else {
              isThinking.value = false;
          }
      });

      recordingStartedUnlisten = await listen('recording-started', (event) => {
        const recordingId = event.payload.recording_id;
        if (!recordings.value.some(r => r.id === recordingId)) {
          localRecordings.value[recordingId] = '';
          recordings.value = [{
            id: recordingId,
            name: recordingId,
            content: '',
            path: '',
            has_git: false
          }, ...recordings.value];
        }
        selectedFile.value = recordingId;
        selectedFiles.value = [recordingId];
        currentEditingFile.value = recordingId;
        isEditingRecording.value = true;
      });

      const recordingsUpdatedUnlisten = await listen('recordings-updated', (event) => {
        loadRecordings();
      });

      const recordingRenamedUnlisten = await listen('recording-renamed', (event) => {
        const { id, new_name } = event.payload;
        console.log(`[Frontend] Recording renamed: ${id} -> ${new_name}`);
        const idx = recordings.value.findIndex(r => r.id === id);
        if (idx !== -1) {
          recordings.value[idx].name = new_name;
          // Trigger reactivity
          recordings.value = [...recordings.value];
        }
      });

      onUnmounted(() => {
        if (documentUpdateUnlisten) documentUpdateUnlisten();
        if (recordingStartedUnlisten) recordingStartedUnlisten();
        if (agentStatusUnlisten) agentStatusUnlisten();
        if (recordingsUpdatedUnlisten) recordingsUpdatedUnlisten();
        if (recordingRenamedUnlisten) recordingRenamedUnlisten();
        stopMicMonitoring();
      });
    });

    return {
      files,
      recordings,
      isRecording,
      isPaused,
      isThinking,
      documentContent,
      micVolume,
      selectedFile,
      selectedFiles,
      currentEditingFile,
      isEditingRecording,
      leftPanelWidth,
      // Rename Modal State
      isRenameModalOpen,
      renameTargetName,
      handleRenameConfirm,
      // ...
      loadFiles,
      loadRecordings,
      refreshData,
      handleFileUpload,
      selectFile,
      selectRecording,
      createNewDocument,
      deleteFiles,
      handleDrop,
      renameFile,
      exportFile,
      startRecording,
      togglePause,
      stopRecording,
      resetDocument,
      handleDocumentUpdate,
      saveCurrentDocument,
      startResize,
      saveCurrentDocument,
      startResize,
      navigateBack,
      isSidebarCollapsed,
      toggleSidebar
    };
  }
};
</script>

<style scoped>
/* Gen Jyuu Gothic font */
@font-face {
  font-family: 'Gen Jyuu Gothic';
  src: url('@/assets/fonts/GenJyuuGothic-Medium.ttf') format('truetype');
  font-weight: 500;
  font-style: normal;
}

.workspace-view {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: #f8f9f3;
  color: #5e6c53;
  user-select: none; /* Globally disable selection, re-enable in editor */
  -webkit-user-select: none;
}

.workspace-content {
  display: flex;
  flex: 1;
  overflow: hidden;
}

/* Resizer */
.resizer {
  width: 4px;
  background: #c1c989;
  cursor: col-resize;
  transition: background 0.15s;
  position: relative; /* Required for absolute positioning of child button */
}

.resizer:hover {
  background: #9fb296;
}

.sidebar-toggle-btn {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 28px;
  height: 28px;
  background: #f8f9f3;
  border: 1px solid #c1c989;
  border-radius: 50%;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  color: #5e6c53;
  z-index: 10;
  transition: all 0.2s;
  box-shadow: 0 2px 6px rgba(0,0,0,0.15);
  opacity: 0.8; /* More visible by default */
}

/* Show button fully when hovering resizer or button */
.resizer:hover .sidebar-toggle-btn,
.sidebar-toggle-btn:hover {
  opacity: 1;
}

/* When collapsed, resizer is at the edge, keep button visible or accessible */
/* Actually, when width is 0, sidebar is hidden. Resizer should still be visible? */
/* We need to ensure sidebar element hides overflow when collapsed */


/* Main panel */
.main-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: #f8f9f3;
  min-height: 0; /* Important for flex children */
  min-width: 0;
}

/* Canvas area */
.canvas-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: #ffffff;
  overflow: hidden;
  min-height: 0; /* Important for flex children to scroll properly */
}

.canvas-area > * {
  flex: 1;
  min-height: 0;
  min-width: 0;
}
</style>
