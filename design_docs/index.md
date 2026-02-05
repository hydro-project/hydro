---
---
# Design Docs

{% for page in site.pages %}
{% assign design_doc_file = page.url | remove_first: "/design_docs/" %}
{% if page.url == design_doc_file %}{% elsif design_doc_file == "" %}{% else %}
- [{{ page.name }}]({{ design_doc_file }})
{% endif %}
{% endfor %}
