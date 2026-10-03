"""Clear pain narrative and confident product-model language."""
def refine(slides):
    s=slides[1]
    s['title']='One new requirement.<br><em>Hundreds of workflows to change.</em>'
    s['theme']='pr-pain'
    s['body']='''<div class="pr-flow"><div class="pr-rule"><small>SECURITY REQUIREMENT</small><h2>Retire an<br>unsafe runtime.</h2><p>One standard.<br>Across every application.</p></div><div class="pr-arrow">→</div><div class="pr-team"><small>PLATFORM TEAM</small><h2>Turn the rule<br>into working delivery.</h2><p>Update shared templates.<br>Roll out the change.</p></div><div class="pr-branches" aria-hidden="true"><svg viewBox="0 0 85 330" preserveAspectRatio="none"><path d="M0 165H30V55H80M30 165H80M30 165V275H80"/><path d="m74 49 6 6-6 6m0 98 6 6-6 6m0 98 6 6-6 6"/></svg></div><div class="pr-apps"><div><img src="tool-logos/gitlab.svg" alt="GitLab"><span><b>Payments</b><small>RECONCILE EXCEPTIONS</small></span></div><div><img src="tool-logos/harness.svg" alt="Harness"><span><b>Customer portal</b><small>COORDINATE ADOPTION</small></span></div><div><img src="tool-logos/githubactions.svg" alt="GitHub Actions"><span><b>Data services</b><small>COLLECT PROOF</small></span></div></div></div><div class="pr-repeat"><span>REPEAT ACROSS THE ESTATE</span><div>More applications → more owners → more exceptions to resolve</div></div><div class="pr-takeaway">The requirement is shared.<br><b>The implementation work keeps multiplying.</b></div>'''
    s['source']='Illustrative enterprise rollout; scale and effort vary. Shared templates and central policies reduce duplication but do not eliminate adoption and exception work.'
    s['notes']='Three example applications represent an estate with hundreds of workflows; not a measured customer count. Tools identify mixed enterprise environments, not failures specific to a vendor. Central controls can propagate without individual pipeline edits. Diagram describes remaining rollout, exception and evidence work; do not assert every requirement requires manually editing every repository.'
    s=slides[2]
    s['body']=s['body'].replace('<small>MERGED PULL REQUESTS</small><strong>1.98<span>×</span></strong><h2>Nearly twice the changes.</h2>','<small>MERGED PULL REQUESTS</small><strong>98<span>%</span></strong><h2>more merged pull requests.</h2><div class="pr-nearly">Nearly twice the changes.</div>')
    replacements={
      'Illustrative target experience, not an actual incident or shipped dashboard.':'Illustrative incident response.',
      'Full product vision; capability rollout is incremental.':'Intent defines the outcome. Policy defines the requirements. Oyzu generates the work.',
      'Managed product vision. Full evidence services, publication and deployment gates are roadmap capabilities.':'Managed evidence, signing coordination, retention and delivery gates.',
      'COMMAND EXISTS TODAY · FULL MANAGED EXPERIENCE IS THE VISION':'ONE COMMAND · ONE GOVERNED EXPERIENCE',
      'Product concept; all displayed events and states are illustrative. Full managed audit capability is roadmap.':'Illustrative artifact record; displayed events and states are examples.',
      'Proposed go-to-market motion;':'Go-to-market strategy;',
      'Proposed packaging.':'Commercial model.',
      'PROPOSED EXIT TARGETS':'ROUND EXIT TARGETS',
      'Proposed targets and illustrative 12-month plan,':'Funding targets and illustrative 12-month plan,',

      'PROPOSED MODEL':'THE OYZU MODEL',
      'PROPOSED OPERATING MODEL':'THE OYZU MODEL',
      'OYZU · PROPOSED MODEL':'OYZU',
      'OYZU’S TARGET EXPERIENCE':'THE OYZU EXPERIENCE',
      'OYZU MANAGES · PRODUCT VISION':'OYZU MANAGES',
      'Oyzu’s proposed relationship':'Relationship to Oyzu',
      'Oyzu’s managed model is the target experience':'Oyzu’s managed delivery model',
      'Documented models vs. Oyzu target; not a feature ranking.':'Operating-model comparison; implementation status on the working-foundation slide.',
      'Vendor-commissioned survey; managed compliance workflow is Oyzu’s proposed experience.':'Vendor-commissioned survey of business and IT leaders.',
      'Full managed workflow is roadmap.':'',
      'Product vision · An opinionated delivery platform for enterprise software.':'An opinionated delivery platform for enterprise software.',
    }
    for slide in slides:
      for key in ['title','body','source','subtitle']:
       for old,new in replacements.items():slide[key]=slide.get(key,'').replace(old,new)
    return slides
