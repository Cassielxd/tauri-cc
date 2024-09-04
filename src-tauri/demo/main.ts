import "@demo/def.ts";

let worker = null;
globalThis.onload = async (e: Event): Promise<void> => {
  console.log("onload");
};


