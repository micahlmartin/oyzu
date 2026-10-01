// Negative dependency lifecycle script: must fail during isolated execution.
await fetch('https://downloads.example.invalid/browser.zip');
