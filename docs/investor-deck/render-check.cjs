const path=require('node:path');
const fs=require('node:fs');
const {pathToFileURL}=require('node:url');
fs.mkdirSync(path.join(__dirname,'screenshots'),{recursive:true});
const {chromium}=require('playwright');
(async()=>{
 const browser=await chromium.launch();
 const page=await browser.newPage({viewport:{width:1440,height:900},deviceScaleFactor:1});
 const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(pathToFileURL(path.join(__dirname,'index.html')).href);
 await page.addStyleTag({content:'.controls{display:none!important}'});
 const count=await page.locator('.slide').count();let layout=[];
 for(let i=0;i<count;i++){
  await page.evaluate(n=>show(n),i);
  const bounds=await page.locator('.slide.active').evaluate(e=>{
   const c=e.querySelector('.content'),src=e.querySelector('.source').getBoundingClientRect(),r=e.getBoundingClientRect();
   const elements=[...c.children];let max=Math.max(...elements.map(x=>x.getBoundingClientRect().bottom));
   const outside=[...c.querySelectorAll('*')].filter(x=>{const b=x.getBoundingClientRect();return b.width>0&&(b.right>r.right-20||b.left<r.left)}).map(x=>x.className).slice(0,10);
   return {bottom:max,source:src.top,height:r.height,outside};
  });
  if(bounds.bottom>bounds.source-10||bounds.outside.length)layout.push({slide:i+1,...bounds});
  await page.screenshot({path:path.join(__dirname,'screenshots/slide-')+String(i+1).padStart(2,'0')+'.png'});
 }
 const images=await page.locator('img').evaluateAll(xs=>xs.filter(x=>!x.complete||x.naturalWidth===0).map(x=>x.getAttribute('src')));
 await page.evaluate(()=>show(0));await page.keyboard.press('ArrowRight');const navigation=await page.locator('#count').textContent();
 await page.emulateMedia({media:'print'});
 const print=await page.locator('.slide').evaluateAll(ss=>ss.flatMap((s,i)=>{
  const c=s.querySelector('.content'),src=s.querySelector('.source').getBoundingClientRect();
  const max=Math.max(...[...c.children].map(x=>x.getBoundingClientRect().bottom));
  return max>src.top-10||getComputedStyle(s).flexDirection!=='column'?[{slide:i+1,overlap:max-src.top}]:[];
 }));
 await page.pdf({path:path.join(__dirname,'oyzu-investor-deck.pdf'),printBackground:true,preferCSSPageSize:true});
 await page.pdf({path:path.join(__dirname,'oyzu-investor-deck-main.pdf'),printBackground:true,preferCSSPageSize:true,pageRanges:'1-20'});
 await page.emulateMedia({media:'screen'});await page.setViewportSize({width:390,height:844});
 let mobile=[];for(let i=0;i<count;i++){await page.evaluate(n=>show(n),i);if(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth))mobile.push(i+1)}
 console.log(JSON.stringify({count,errors,images,navigation,layout,print,mobile}));await browser.close();
})();
