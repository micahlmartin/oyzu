import re

def clarify(slides):
    s=slides[0]
    s['body']=re.sub(r'<g class="radar-logos">.*?</g>','',s['body'],flags=re.S)
    s['body']=re.sub(r'<g class="radar-verified">.*?</g>','',s['body'],flags=re.S)
    s['body']=re.sub(r'<path class="radar-sector"[^>]+/>','',s['body'])
    s['body']=s['body'].replace('<path d="M25 285H525M275 35V535"/>','')
    s['body']=s['body'].replace('<path class="radar-route" d="M275 285L348 224L401 179L455 134"/>','<path class="radar-route" d="M275 285L455 134"/><path class="radar-route" d="m443 134 12 0 0 12"/>')
    s['body']=re.sub(r'<g class="radar-check">.*?</g>','',s['body'],flags=re.S)
    s['notes']+=' Simplified radar: three stages and one outward path; no live verification indicators.'
    proof=next(x for x in slides if x['label']=='THE WORKING FOUNDATION')
    proof['title']='The full delivery lifecycle.<br><em>A working build foundation.</em>'
    proof['theme']='fc-lifecycle'
    stages=[('Build','Resolve tools.<br>Produce artifacts.'),('Scan','Assess dependencies<br>and artifact risk.'),('Attest','Bind identity and<br>evidence to artifacts.'),('Publish','Deliver approved<br>artifacts and proof.'),('Verify','Check evidence and<br>delivery requirements.')]
    proof['body']='''<div class="fc-stages">'''+''.join('<div><small>'+str(i+1).zfill(2)+'</small><h2>'+name+'</h2><p>'+desc+'</p></div>' for i,(name,desc) in enumerate(stages))+'''</div><div class="fc-evidence"><div><small>WORKING TODAY · DOCUMENTED NATIVE FIXTURES</small><h3>Build behavior with retained evidence.</h3><p>Passing tests retain artifacts and reports.<br>Assertion or coverage failures retain failure evidence.<br>Repeated qualified Yarn builds produced identical archive digests.</p><a href="https://github.com/micahlmartin/oyzu/actions/runs/36863353630">Inspect the historical CI evidence →</a></div><div><small>WHAT THIS ROUND ADVANCES</small><h3>Managed, governed publication.</h3><p>Central policy, evidence services and publication gates.<br>Broader delivery integrations build on that foundation.</p><b>One product owns the supported delivery mechanics.</b></div></div>'''
    proof['source']='Lifecycle labels describe product capabilities, not CLI command names. Current proof is historical and scoped; funded milestones appear in the operating plan.'
    proof['notes']+=' Replaces CLI syntax with an understandable lifecycle. Scan, attest, publish and verify labels are conceptual capability areas, not claims of shipped commands. Verify can gate multiple stages; the illustration is not a strict technical ordering.'
    return slides
