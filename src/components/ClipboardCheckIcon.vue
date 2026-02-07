<template>
  <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" class="clipboard-icon">
    <g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round">
        <path d="M12 3h7v18h-14v-18h7Z" stroke-width="2" />
        <path d="M14.5 3.5v3h-5v-3" />
        <path d="M9 13l2 2l4 -4" stroke-width="2" stroke-dasharray="12">
            <!-- Trigger animation when class 'animate' is present or just run it? 
                 User said "icon... transition". The SVG from line-md has auto-run animation. 
                 To control it, we might want to trigger it on expand/collapse?
                 For now, let's keep it auto-running or loop it. 
                 The source had fill="freeze", so it runs once. 
                 Let's make it loop or run on mount. 
            -->
            <animate attributeName="stroke-dashoffset" dur="0.6s" fill="freeze" values="12;0" />
        </path>
    </g>
  </svg>
</template>

<script>
export default {
  name: 'ClipboardCheckIcon'
}
</script>

<style scoped>
.clipboard-icon {
    /* Optional: Add hover effects or color */
}
</style>
