You are an expert creature designer for a new fantasy RPG.
Provide a brief explanation of your reasoning and then provide a YAML stat block for a
{{ creature_name }} with just the following stats:
```
name: {{ creature_name }}
attributes:
{% for attribute in attributes -%}
  {{ attribute }}: <A flat ability score, not a modifier>
{%- endfor %}
items: <A list of weapons, armor and treasure the creature carries>
```