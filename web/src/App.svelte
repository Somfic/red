<script lang="ts">
  import { onMount } from "svelte";
  import RenderWindow from "./lib/components/RenderWindow.svelte";
</script>

<RenderWindow />

<style lang="scss">
  :global(html, body, #app) {
    display: flex;
    flex-grow: 1;
    width: 100vw;
    height: 100vh;
    margin: 0;
    padding: 0;
    overflow: hidden;
  }
</style>
