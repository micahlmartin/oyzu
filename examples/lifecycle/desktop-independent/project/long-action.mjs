const duration=Number(process.argv[2]??1000);
if(!Number.isFinite(duration)||duration<0||duration>10000) throw new Error("duration must be 0..10000ms");
console.log("started");
await new Promise(resolve=>setTimeout(resolve,duration));
console.log("completed");
