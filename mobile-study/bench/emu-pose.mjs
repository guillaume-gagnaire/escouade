// Oriente l'appareil photo virtuel de l'émulateur Android (la scène 3D, -camera-back virtualscene) : écrit une macro
// d'automatisation (le protobuf de l'émulateur : 4 octets de longueur puis le message) qui donne au modèle physique la
// rotation voulue, en degrés autour de x (tangage), y (lacet) et z (roulis), puis la joue par la console.
// C'est ainsi que l'appareil photo d'une maquette « vise » l'affiche du QR code (-virtualscene-poster wall=pair.png).
//
//   node mobile-study/bench/emu-pose.mjs <avd-dir> <x> <y> <z> [<px> <py> <pz>]   (rotation en degrés, position en mètres)
import { execFileSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';

const [avdDir, x, y, z, px, py, pz] = process.argv.slice(2);
const frame = (body) => {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(body.length);
  return Buffer.concat([len, body]);
};
const floats = (v) => {
  const b = Buffer.alloc(4 * v.length);
  v.forEach((f, i) => b.writeFloatLE(f, i * 4));
  return b;
};
const lenField = (tag, body) => Buffer.concat([Buffer.from(tag), Buffer.from([body.length]), body]);
// En-tête : version 2, état initial vide.
const header = frame(Buffer.from([0x08, 0x02, 0x12, 0x00]));
// RecordedEvent { delay = 1 ; physical_model (champ 50) { type = ROTATION (1) ; current_value { data: [x, y, z] } } }
const value = lenField([0x0a], floats([Number(x), Number(y), Number(z)]));
const model = Buffer.concat([Buffer.from([0x08, 0x01]), lenField([0x12], value), lenField([0x1a], value)]);
const event = frame(Buffer.concat([Buffer.from([0x08, 0x01]), lenField([0x92, 0x03], model)]));
// La position (type POSITION, 0), si elle est donnée : la scène met l affiche du mur vers (-0.807, 0.320, 5.316).
const posValue = lenField([0x0a], floats([Number(px ?? 0), Number(py ?? 0), Number(pz ?? 0)]));
const posModel = Buffer.concat([Buffer.from([0x08, 0x00]), lenField([0x12], posValue), lenField([0x1a], posValue)]);
const posEvent = px === undefined ? Buffer.alloc(0) : frame(Buffer.concat([Buffer.from([0x08, 0x01]), lenField([0x92, 0x03], posModel)]));
const end = frame(Buffer.from([0x08, 0x01]));
writeFileSync(join(avdDir, 'console_out', 'pose.txt'), Buffer.concat([header, posEvent, event, end]));
console.log(execFileSync('adb', ['emu', 'automation', 'play', join(avdDir, 'console_out', 'pose.txt')], { encoding: 'utf8' }).trim());
