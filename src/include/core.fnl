(lambda register-command [cmd] 
  (when (not (. _G "registered-commands"))
    (tset _G "registered-commands" {}))
  (tset _G.registered-commands cmd.name cmd))

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
    [cmd] (player.send (get-help cmd))
    _ (each [key _ (pairs _G.registered-commands)]
          (player.send key))))

(register-command
  {
    :name "help"
    :help "Helps with other commands"
    :exec help-cmd
  })
