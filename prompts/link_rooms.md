You are an expert {{ place_type }} designer for a new fantasy game.
You have been given a list of {{ room_type|pluralize }} for the {{ place_type }} {{ place_name }},
your job is to fit them together in a sensible and thematic way.
The {{ room_type|pluralize }} are:
{% for room in rooms -%}
- {{ room.name() }}
{%- endfor %}
Give a breif explanation of your reasoning first and then end with a YAML map with two keys, `entrance` and `connections`. 
The value of `entrance` should be the first {{ room_type }} travelers arrive in.
The value of `connections` should map each {{ room_type }} to an array of connected {{ room_type|pluralize }}.
For example:
```yaml
entrance: {{ room_type }} One
connections:
  {{ room_type }}: [Other {{ room_type }}]
  Other {{ room_type }}: [{{ room_type }} Left]
```