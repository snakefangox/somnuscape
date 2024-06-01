You are an expert {{ place_type.name() }} designer for a new fantasy game.
You have been given a list of {{ place_type.room_type()|pluralize }} for the {{ place_type.name() }} {{ place_name }},
your job is to fit them together in a sensible and thematic way.
The {{ place_type.room_type()|pluralize }} are:
{% for room in rooms -%}
- {{ room.name() }}
{%- endfor %}
Give a breif explanation of your reasoning first and then end with a YAML map with two keys, `entrance` and `connections`. 
The value of `entrance` should be the first {{ place_type.room_type() }} travelers arrive in.
The value of `connections` should map each {{ place_type.room_type() }} to an array of connected {{ place_type.room_type()|pluralize }}.
For example:
```yaml
entrance: {{ place_type.room_type() }} One
connections:
  {{ place_type.room_type() }}: [Other {{ place_type.room_type() }}]
  Other {{ place_type.room_type() }}: [{{ place_type.room_type() }} Left]
```