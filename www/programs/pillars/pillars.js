import * as math from 'mathjs';
import { debugCanvas } from '../base'

let hallWidth = 1.0;
let hallHeight = 14.0;
let cellSize = 2;
let vertices = [];
let ctx = debugCanvas.getContext("2d");
let range = 10;
let size = range * (cellSize);
let offset = -(size/2);
let factor = Math.min(canvas.width / size, canvas.height / size);

function s(a) {
    return (a-offset) * factor;
}

function wall(x, z, x2, z2) {
    ctx.fillStyle = 'red';
    ctx.strokeStyle = 'red';
    ctx.moveTo(s(x), s(z));
    ctx.lineTo(s(x2), s(z2)); 
    ctx.stroke();

    return [
        x, 0, z,
        x, hallHeight, z,
        x2, 0, z2,
        x, hallHeight, z,
        x2, 0, z2,
        x2, hallHeight, z2,
    ]
}

function floor(x, z, x2, z2) {
    return [
        x,  0, z,
        x2, 0, z,
        x2, 0, z2,
        x2, 0, z2,
        x,  0, z2,
        x,  0, z,
    ]
}

const seed = Math.random() * 99999999;

function isCellClosed(x, z) {
    return (x < 0 || z < 0) || (x >= range || z >= range);
        // || x % 3 == 0 || z % 5 == 0;
        //  || (seed & (x * 3 ^ z) & (z*3 ^ 3));
}

function cellPosition(cell) {
    return {
        x: offset + (cell.x * cellSize),
        z: offset + (cell.z * cellSize)
    };
}

function hallPosition(cell) {
    let pos = cellPosition(cell);
    let hallOffset = (cellSize-hallWidth)/2;
    return {x: pos.x + hallOffset, z: pos.z + hallOffset}
}

function degToRad(number) {
    return number * math.pi / 180.0;
}

function passage(cell, targetCell) {
    let pos = cellPosition(cell);
    let center = {x: pos.x + cellSize / 2, z: pos.z + cellSize / 2}
    let hallOffset = hallWidth / 2;
    let direction = {x: targetCell.x - cell.x, z: targetCell.z - cell.z};

    let angle = 0;
    if (direction.x > 0) {
        angle = 90;
    } else if (direction.x < 0) {
        angle = 270;
    } else if(direction.z < 0) {
        angle = 180;
    }

    const rotation = math.rotationMatrix(degToRad(-angle));
    const posA = math.multiply(rotation, [-hallOffset, hallOffset]);
    const posA2 = math.multiply(rotation, [-hallOffset, hallWidth]);
    const posB = math.multiply(rotation, [hallOffset, hallOffset]);
    const posB2 = math.multiply(rotation, [hallOffset, hallWidth]);

    let vertices = [
        ...wall(center.x + posA.get([0]), center.z + posA.get([1]), center.x + posA2.get([0]), center.z + posA2.get([1])),
        ...wall(center.x + posB.get([0]), center.z + posB.get([1]), center.x + posB2.get([0]), center.z + posB2.get([1])),
    ];
    return vertices;
}

function closedWall(cell, targetCell) {
    let hallPos = hallPosition(cell);
    if (cell.z == targetCell.z) {
        let x = (targetCell.x > cell.x) ? hallPos.x + hallWidth : hallPos.x;
        return wall(x, hallPos.z, x, hallPos.z + hallWidth);
    } else {
        let z = (targetCell.z > cell.z) ? hallPos.z + hallWidth : hallPos.z;
        return wall(hallPos.x, z, hallPos.x + hallWidth, z);
    }
}

for (let x = 0; x < range; x++) {
    for (let z = 0; z < range; z++) {
        let xPos = offset + (x * cellSize);
        let zPos = offset + (z * cellSize);
        ctx.fillStyle = 'blue';
        ctx.fillRect(s(xPos), s(zPos), cellSize * factor, cellSize * factor);
        
        let hallX = xPos + (cellSize-hallWidth)/2;
        let hallZ = zPos + (cellSize-hallWidth)/2;
        ctx.fillStyle = 'black';
        ctx.fillRect(s(hallX), s(zPos), hallWidth * factor, cellSize * factor);
        ctx.fillRect(s(xPos), s(hallZ), cellSize * factor, hallWidth * factor);

        const cell = {x: x, z: z};
        const left = {x: cell.x - 1, z: cell.z};
        const right = {x: cell.x + 1, z: cell.z};
        const top = {x: cell.x, z: cell.z + 1};
        const bottom = {x: cell.x, z: cell.z - 1};
        const sides = [left, right, top, bottom];
        if (!isCellClosed(cell.x, cell.z)) {
            for (let side of sides) {
                if (isCellClosed(side.x, side.z)) {
                    vertices.push(...closedWall(cell, side));
                } else {
                    vertices.push(...passage(cell, side));
                }
            }
            vertices.push(...floor(hallX, zPos, hallX+hallWidth, zPos+cellSize))
            vertices.push(...floor(xPos, hallZ, xPos+cellSize, hallZ+hallWidth))
        }
    }
}

export default {
    vertexShader: "programs/pillars/vertex.glsl",
    fragmentShader: "programs/pillars/fragment.glsl",
    vertices: vertices,
    vertsPerPoly: 3,
};