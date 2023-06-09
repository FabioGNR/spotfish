const debugCanvas = document.getElementById("debug-canvas");
const canvasRect = debugCanvas.getBoundingClientRect();
debugCanvas.width = canvasRect.width;
debugCanvas.height = canvasRect.height;

export {
    debugCanvas,
};