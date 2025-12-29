<script lang="ts">
    import { onMount } from "svelte";

    let loading = $state(true);

    onMount(async () => {
        const wasm = await import("../wasm/rendering.js");
        await wasm.default();
        loading = false;
    });
</script>

<div class="canvas">
    <div class="loading" style:display={loading ? "flex" : "none"}>Loading...</div>
    <canvas id="game-canvas"></canvas>
</div>

<style lang="scss">
    .canvas {
        position: relative;
        display: flex;
        flex-grow: 1;
        width: 100%;
        height: 100%;
    }

    canvas {
        flex-grow: 1;
    }

    .loading {
        position: absolute;
        inset: 0;
        z-index: 10;
        display: flex;
        align-items: center;
        justify-content: center;
        background-color: #111;
        color: #888;
        font-family: system-ui, sans-serif;
    }
</style>
