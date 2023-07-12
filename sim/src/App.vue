<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { initState, state } from './ts/canvas';
const container = ref<HTMLDivElement | null>(null);
onMounted(() => {
  if (container.value !== null) {
    initState(container.value);
  }
})

const toFullscreen = () => {
  (container.value as HTMLDivElement).requestFullscreen();
  (container.value as HTMLDivElement).onfullscreenchange = () => {
    (container.value as HTMLDivElement).style.cursor = (container.value as HTMLDivElement).style.cursor === "" ? "none" : "";
  }
}

const openWindow = () => {
  let w = window.open("http://localhost:5173/", '_blank', 'left=100,top=50,width=830,height=750',) as Window;
}

</script>

<template>
  <!-- <div id="info">
    <button @click="toFullscreen">fullscreen</button>
  </div> -->
  <div ref="container"></div>
</template>

<style scoped>
#info {
  position: absolute;
  top: 0px;
  width: 100%;
  padding: 5px;
  box-sizing: border-box;
  text-align: center;
  z-index: 10;
}
</style>
