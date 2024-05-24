(lambda register-command [cmd] 
  (when (not (. _G "registered-commands"))
    (tset _G "registered-commands" {}))
  (tset _G.registered-commands cmd.name cmd))

(lambda run-command [cmd-name world player args]
  (let [cmd (?. _G.registered-commands cmd-name)]
    (if cmd
      (cmd.exec world player args)
      (.. "Invalid command: " cmd-name))))

(lambda deep-print [?val]
    (if (= (type ?val) :table)
      (each [key value (pairs ?val)]
        (print key)
        (deep-print value))
      (print ?val)))

(lambda get-help [?cmd]
  (let [cmd (?. _G.registered-commands ?cmd)]
    (if cmd
      (.. cmd.name ": " cmd.help)
      (.. "Command '" ?cmd "' does not exist"))))

(lambda help-cmd [_world player args]
  (case args
    [cmd] (get-help cmd)
    _ (.. "Listing all commands\nGet help for a specific command with `help <command name>`\nValid commands:\n"
        (table.concat
          (icollect [key _ (pairs _G.registered-commands)] (.. "\n" key))))))

(register-command
  {
    :name "help"
    :help "Helps with other commands"
    :exec help-cmd
  })

(lambda init-globals [globals]
  (each [key val (pairs globals)]
    (tset _G key val)))

(init-globals
  {
    :register-command register-command
    :deep-print deep-print
    :run-command run-command
  })
