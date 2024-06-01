You are an expert {{ place_type.name }} designer for a new fantasy game.
You have been given a list of {{ place_type.room_types_pural }} for the {{ place_type.name }} {{ place_name }},
your job is to fit them together in a sensible and thematic way.

Give a brief explanation of your reasoning first and then end with a YAML map with two keys, `entrance` and `connections`. 
The value of `entrance` should be the first {{ place_type.room_type }} travelers arrive in.
The value of `connections` should map each {{ place_type.room_type }} to an array of connected {{ place_type.room_types_pural }}.
For example:
```yaml
entrance: {{ place_type.room_type }} One
connections:
  {{ place_type.room_type }}: [Other {{ place_type.room_type }}]
  Other {{ place_type.room_type }}: [{{ place_type.room_type }} Left]
```

The {{ place_type.room_types_pural }} are:
{% for room in rooms -%}
- {{ room.name }}
{%- endfor %}
