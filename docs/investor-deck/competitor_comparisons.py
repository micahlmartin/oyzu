"""Scenario-based comparisons; product maturity remains on the foundation slide."""
from html import escape


COMPARISONS = [
    dict(name='Harness', title='Who maintains the delivery logic?',
         scenario='A new security scanner becomes mandatory.',
         strength='Shared templates centralize logic. Pinned versions and input changes can still require upgrades across consuming pipelines.',
         overlap='Central standards, reusable automation and controlled execution.',
         customer='Define the scanner steps, template inputs and integrations; maintain the delivery logic behind the guardrails.',
         oyzu='Declare the scanner requirement in policy. Oyzu resolves the supported delivery steps and records their evidence.',
         takeaway='The distinction is ownership of delivery logic, not the existence of policy.',
         fit='Adoption path: retain existing execution infrastructure while moving supported delivery logic into Oyzu.',
         sources=[('Harness templates', 'https://developer.harness.io/docs/platform/Templates/template'), ('Harness policy as code', 'https://developer.harness.io/docs/category/policy-as-code/'), ('Version adoption and reconciliation', 'https://developer.harness.io/docs/platform/pipeline-faq')]),
    dict(name='GitLab', title='Central enforcement is already possible.',
         scenario='Require a security check across many projects.',
         strength='Pipeline execution policies enforce shared CI/CD jobs across projects, including projects without their own CI configuration.',
         overlap='Central enforcement, security checks and delivery evidence.',
         customer='Maintain the shared CI job definitions and their compatibility with application build and delivery behavior.',
         oyzu='Resolve application intent and organization policy into the delivery plan, including approved tools from local development onward.',
         takeaway='Oyzu brings the governed model to the developer command and the delivery plan.',
         fit='Adoption path: retain GitLab for source collaboration and execution; shift supported delivery mechanics into Oyzu.',
         sources=[('GitLab pipeline execution policies', 'https://docs.gitlab.com/user/application_security/policies/pipeline_execution_policies/')]),
    dict(name='Bazel', title='A reproducible build is a foundation.',
         scenario='Approve a new compiler across independent repositories.',
         strength='Build rules and platform-aware toolchain resolution select tools for reproducible, explicitly modeled builds.',
         overlap='Deterministic planning, explicit tools and repeatable execution.',
         customer='Maintain rule and toolchain definitions, register approved versions and coordinate their adoption across workspaces.',
         oyzu='Resolve approved tools through organizational policy and catalogs, connecting the chosen tools to delivery controls and evidence.',
         takeaway='Tool selection becomes part of an organization-wide delivery model.',
         fit='Different layer: Bazel models the build graph. Oyzu owns the supported delivery path and its organizational controls.',
         sources=[('Bazel toolchains', 'https://bazel.build/extending/toolchains'), ('Bazel build model', 'https://bazel.build/concepts/build-ref')]),
    dict(name='Pants', title='Less build configuration is not the whole job.',
         scenario='Standardize approved tools for developers and AI agents.',
         strength='Dependency inference and isolated execution reduce build configuration and make supported tasks repeatable.',
         overlap='Convention-driven developer experience and managed build tools.',
         customer='Choose and maintain repository configuration, tool versions and extensions; connect them to company approval and delivery controls.',
         oyzu='Make approved sources, tool selection and access rules part of the governed path, with policy and evidence spanning delivery.',
         takeaway='Convenient setup and organizational approval belong in the same experience.',
         fit='Different responsibility: Pants simplifies repository tasks. Oyzu connects the developer experience to enterprise delivery policy.',
         sources=[('How Pants works', 'https://www.pantsbuild.org/stable/docs/introduction/how-does-pants-work'), ('Pants lockfiles', 'https://www.pantsbuild.org/stable/docs/python/overview/lockfiles')]),
    dict(name='Buck2', title='Explicit toolchains still need an owner.',
         scenario='Move teams onto an approved production toolchain.',
         strength='Reusable rules separate build behavior from tool sourcing. Hermetic toolchains can download and track tools explicitly.',
         overlap='Explicit tool dependencies and controlled execution.',
         customer='Supply and maintain production toolchains, choose approved sources and coordinate changes across repositories.',
         oyzu='Own supported tool acquisition and delivery mechanics as product behavior, governed by shared policy and recorded evidence.',
         takeaway='The product takes on the recurring work around the build engine.',
         fit='Different layer: Buck2 provides build primitives. Oyzu packages the organizational delivery responsibility around supported ecosystems.',
         sources=[('Buck2 toolchains', 'https://buck2.build/docs/concepts/toolchain/'), ('Writing Buck2 toolchains', 'https://buck2.build/docs/rule_authors/writing_toolchains/')]),
]


DETAILS = {
    'Harness': (
        ['Pipeline templates, inputs and OPA guardrails.', 'Stable references can follow updates automatically; pinned versions and changed inputs need adoption or reconciliation.', 'Platform teams maintain pipeline steps and integrations.'],
        ['Template + policy', 'Pipeline execution', 'Stage results']),
    'GitLab': (
        ['Shared CI jobs and pipeline execution policies.', 'Enforce shared jobs centrally across scoped projects.', 'Platform teams maintain CI definitions and application compatibility.'],
        ['Policy + CI jobs', 'Project pipelines', 'Job evidence']),
    'Bazel': (
        ['Build targets, rules and registered toolchains.', 'Publish approved rule/toolchain versions; adopt them across workspaces.', 'Rule owners maintain build definitions and toolchain integrations.'],
        ['Rules + toolchains', 'Build graph', 'Build outputs']),
    'Pants': (
        ['Repository settings, backends and tool versions.', 'Update shared configuration or plugins; coordinate repository adoption.', 'Teams maintain configuration and extensions around inferred tasks.'],
        ['Config + inference', 'Isolated tasks', 'Task outputs']),
    'Buck2': (
        ['Build rules and production toolchain definitions.', 'Distribute toolchain changes and coordinate repository adoption.', 'Rule owners maintain tool sourcing and production toolchains.'],
        ['Rules + toolchains', 'Action graph', 'Build outputs']),
}


def flow(steps):
    return '<div class="cx-flow">' + '<i aria-hidden="true">→</i>'.join('<span>'+escape(x)+'</span>' for x in steps) + '</div>'


def expand_comparisons(slides):
    additions = []
    for item in COMPARISONS:
        name = item['name']
        rows, steps = DETAILS[name]
        ours = ['Application intent, approved tools and organizational policy.',
                'Change shared policy; Oyzu resolves the supported plan and records enforcement.',
                'Oyzu maintains supported delivery mechanics; the organization owns policy and exceptions.']
        labels = ['Teams configure', 'A requirement changes', 'Logic is maintained by']
        strength = 'One governed path connecting approved tools, delivery controls and retained evidence.'
        our_steps = ['Intent + policy', 'Resolved plan', 'Delivery + evidence']
        takeaway = item['takeaway']
        if name == 'Harness':
            strength = 'Generates build, scan and publish steps from application requirements and policy. Teams do not maintain that sequence.'
            ours = ['Application requirements; security selects the scanner, scope and blocking rules.',
                    'Security updates policy. Oyzu includes the required scanner in the next applicable generated plan.',
                    'Oyzu maintains scanner integration code centrally. The organization owns rules and exceptions.']
            our_steps = ['App + scanner policy', 'Generate steps', 'Build → scan → publish']
            takeaway = 'The steps still exist. Oyzu generates the sequence and maintains the integrations.'
        logo = {'Harness':'harness.svg', 'GitLab':'gitlab.svg'}.get(name)
        mark = f'<img src="tool-logos/{logo}" alt="">' if logo else '<span class="cx-monogram">'+escape(name[0])+'</span>'
        table = ''.join(f'<div class="cx-row"><b>{label}</b><p>{escape(left)}</p><p>{escape(right)}</p></div>' for label,left,right in zip(labels,rows,ours))
        body = f'''<div class="cx-scenario">{escape(item['scenario'])}</div>
        <div class="cx-head"><span></span><h2>{mark}{escape(name)}</h2><h2>Oyzu</h2></div>
        <div class="cx-row cx-strength"><b>Starting strength</b><p>{escape(item['strength'])}</p><p>{escape(strength)}</p></div>
        {table}
        <div class="cx-process"><small>HOW WORK FLOWS</small>{flow(steps)}{flow(our_steps)}</div>
        <div class="cx-takeaway">{escape(takeaway)}</div>'''
        source = ' · '.join(f'<a href="{url}">{escape(label)}</a>' for label, url in item['sources'])
        additions.append(dict(label='APPENDIX · COMPARISON', title='Oyzu vs. '+escape(name), body=body, theme='cc-slide cx-slide', subtitle='', source=source,
            notes='Comparison of operating responsibilities, not a claim that the other product cannot be extended. Competitor capabilities checked against official documentation. Oyzu model follows the founder-defined product direction; current implementation and funded milestones are presented on the working foundation slide. Adoption paths describe architecture, not completed connectors. ' + item['fit'] + ' Shared ground: ' + item['overlap']))
    index = next(i for i, slide in enumerate(slides) if slide['label'] == 'APPENDIX · PRIOR ART')
    slides[index:index + 1] = additions
    return slides
