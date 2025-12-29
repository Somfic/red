<script lang="ts">
  import { onMount } from "svelte";

  onMount(async () => {
    const wasm = await import("./lib/wasm/rendering.js");
    await wasm.default();
  });
</script>

<canvas id="game-canvas"></canvas>

<style lang="scss">
  :global(html, body, #app) {
    margin: 0;
    padding: 0;
    overflow: hidden;
    width: 100vw;
    height: 100vh;
  }

  canvas {
    width: 100%;
    height: 100vh;
    display: block;
    outline: none;
  }
</style>
