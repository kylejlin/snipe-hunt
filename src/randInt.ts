export default function randInt(inclMin: number, exclMax: number): number {
  const diff = exclMax - inclMin;
  return inclMin + Math.floor(diff * Math.random());
}
