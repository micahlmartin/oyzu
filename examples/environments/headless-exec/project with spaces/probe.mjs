console.log(JSON.stringify({cwd:process.cwd(),args:process.argv.slice(2),label:process.env.EXAMPLE_LABEL}));
if (process.argv.includes("--fail")) process.exitCode = 7;
