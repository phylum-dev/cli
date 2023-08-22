function runCount() {
  try {
    const c = parseInt(localStorage.getItem("runCount") ?? "0", 10);
    localStorage.setItem("runCount", `${c+1}`);
    return c;
  } catch(_e) {
    // Local Storage Disabled
    return -1;
  }
}

console.log(`Run Count: ${runCount()}`);
