let vertices = [
    // First triangle:
    1.0,  1.0,
    -1.0,  1.0,
    -1.0, -1.0,
    // Second triangle:
    -1.0, -1.0,
    1.0, -1.0,
    1.0,  1.0
];

export default {
    vertexShader: "programs/visualizer/vertex.glsl",
    fragmentShader: "programs/visualizer/fragment.glsl",
    vertices: vertices,
    vertsPerPoly: 2,
};