#!/usr/bin/env node
const fs = require("fs");

function usage() {
  console.error("usage: tcli upper|len|div|words|copy|now");
  return 1;
}

function main(argv) {
  const [cmd, ...args] = argv;
  if (cmd === "upper") {
    console.log(args.join(" ").toUpperCase());
  } else if (cmd === "len" && args.length === 1) {
    console.log(args[0].length);
  } else if (cmd === "div" && args.length === 2) {
    if (!args.every((a) => /^-?\d+$/.test(a))) {
      console.error("error: not a number");
      return 2;
    }
    const [a, b] = args.map(Number);
    if (b === 0) {
      console.error("error: division by zero");
      return 2;
    }
    console.log(`${a} / ${b} = ${Math.trunc(a / b)} remainder ${a % b}`);
  } else if (cmd === "words") {
    console.log(fs.readFileSync(0, "utf8").split(/\s+/).filter(Boolean).length);
  } else if (cmd === "copy" && args.length === 2) {
    let text;
    try {
      text = fs.readFileSync(args[0], "utf8");
    } catch {
      console.error(`error: cannot read ${args[0]}`);
      return 1;
    }
    try {
      fs.writeFileSync(args[1], text.toUpperCase());
    } catch {
      console.error(`error: cannot write ${args[1]}`);
      return 1;
    }
    console.log(`copied ${text.length} characters`);
  } else if (cmd === "now") {
    console.log(`now: ${process.hrtime.bigint()}`);
  } else {
    return usage();
  }
  return 0;
}

process.exitCode = main(process.argv.slice(2));
