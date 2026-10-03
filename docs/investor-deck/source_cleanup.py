"""Keep visible footers for citations; preserve explanatory context in notes."""
import re
from html import unescape


def clean_sources(slides):
    for slide in slides:
        source = slide.get('source', '')
        if source:
            context = unescape(re.sub('<[^>]+>', ' ', source))
            slide['notes'] += ' Source context: ' + context
        links = re.findall(r'<a\b[^>]*>.*?</a>', source)
        slide['source'] = ' · '.join(links)
        if slide['label'] == 'THE DEVELOPER EXPERIENCE':
            slide['source'] = '<br>'.join(
                f'<span id="devex-source-{i}">{i}. {link}</span>'
                for i, link in enumerate(links, 1)
            )
        if slide['label'] == 'WHY NOW':
            slide['source'] += ' · 10,000+ developers · 1,255 teams · Observed within-team comparison of lower and higher AI adoption periods.'
        replacements = {
            'THE OPERATING MODEL': [('Delivery expertise is built in.', 'The intended operating model.')],
            'HOW THE PLAN IS GENERATED': [('GENERATED EXECUTION PLAN', 'INTENDED EXECUTION PLAN')],
            'ONE GOVERNED DEVELOPER EXPERIENCE': [('THE OYZU EXPERIENCE', 'INTENDED OYZU EXPERIENCE')],
            'ORGANIZATIONAL CHANGE MANAGEMENT': [('NEW STANDARD', 'PLANNED ROLLOUT MODEL')],
            'MANAGED ARTIFACT SECURITY': [('ORGANIZATIONAL POLICY', 'TARGET ARCHITECTURE · ORGANIZATIONAL POLICY')],
            'COMPETE ON THE MODEL · INTEGRATE ON ADOPTION': [('APPLICATION INTENT + ORGANIZATIONAL POLICY', 'PLANNED INTEGRATIONS · INTENT + POLICY')],
            'APPENDIX · ECOSYSTEM': [('Shared policy &amp; evidence', 'Planned policy &amp; evidence'), ('Shared policy & evidence', 'Planned policy & evidence')],
            'APPENDIX · ECONOMIC SENSITIVITY': [('LABOR COST PER CROSS-REPOSITORY CHANGE', 'ILLUSTRATIVE LABOR COST PER CHANGE')],
        }
        for old, new in replacements.get(slide['label'], []):
            slide['body'] = slide['body'].replace(old, new)
    return slides
