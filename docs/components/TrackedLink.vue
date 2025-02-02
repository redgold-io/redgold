<!--<template>-->
<!--  <a-->
<!--      :href="link"-->
<!--      target="_blank"-->
<!--      @click="handleClick"-->
<!--      class="text-blue-500 hover:text-blue-700 underline"-->
<!--  >-->
<!--    {{ text }}-->
<!--  </a>-->
<!--</template>-->

<!--<script setup lang="ts">-->
<!--interface Props {-->
<!--  link: string-->
<!--  text: string-->
<!--  eventId?: string // Optional Twitter event ID-->
<!--}-->

<!--const props = defineProps<Props>()-->

<!--const handleClick = () => {-->
<!--  // @ts-ignore since twq might not be recognized by TypeScript-->
<!--  if (window.twq && props.eventId) {-->
<!--    // @ts-ignore-->
<!--    window.twq('event', props.eventId, {});-->
<!--  }-->
<!--}-->
<!--</script>-->