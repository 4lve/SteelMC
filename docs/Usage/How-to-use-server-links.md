# How to add server links

At first what are server links, these give you the option to add in the pause screen of the user links which he can
click. For example the own marketplace, website of the server, etc.

There are two things which can be used that are build in options, what will be quick and but not much configuration
options available. The other option are TextComponents there you can create and style like you want.

## Enable server links

At first server links needed to be activated this will be done then you add to your steel_config.json5 this text:

```json
server_links: {
// Enable server links feature
  enable: true,
},
```

This needs to be added in the root part of the config, also to deactivate temporarily you can set enable to false.

## Build-in server links

You have 10 build in types for server links these are:

```
bug_report = 0,
community_guidelines = 1,
support = 2,
status = 3,
feedback = 4,
community = 5,
website = 6,
forums = 7,
news = 8,
announcements = 9,
```

The only special case is `bug_report` this will be also displayed then the server breaks or throws an exception or send
malformed data to the client.

This can be used like this:
```json
{
  label: "bug_report",
  url: "https://github.com/4lve/SteelMC/issues"
}
```

and a full example:
```json
server_links: {
  // Enable server links feature
  enable: true,
  // List of links to display in the server menu
  links: [
    // Built-in link type (simple string label)
    {
      label: "bug_report",
      url: "https://github.com/4lve/SteelMC/issues"
    }
  ]
},
```

## Custom server links
These are TextComponent So you have a lot more functionality included here for the design with custom text and color.
This will look like this:
```json
{
    label: {
        text: "Visit the SteelMC Discord",
        color: "blue",
        bold: true
    },
    url: "https://discord.gg/suSXXNdVSf"
}
```
### Additional resources
In the web you can find many more tutorials about TextComponent and how to use them correctly

## Full example config
```json
{
    $schema: "https://raw.githubusercontent.com/4lve/SteelMC/refs/heads/master/package-content/schema.json5",
    // Server port
    server_port: 25565,
    // World seed for generating the world, empty string means random seed
    seed: "",
    // Maximum number of players allowed on the server
    max_players: 20,
    // Maximum view distance in chunks
    view_distance: 10,
    // Maximum simulation distance in chunks
    simulation_distance: 10,
    // Whether to use Mojang's authentication service
    online_mode: true,
    // Whether to enable encryption for client-server communication
    encryption: true,
    // Message of the day displayed in server lists
    motd: "A Steel Server",
    // Whether to use a custom favicon for the server
    use_favicon: true,
    // Path to the favicon file (PNG format, 64x64 pixels)
    favicon: "config/favicon.png",
    // Whether to enforce secure chat
    enforce_secure_chat: false,
    // Compression settings
    compression: {
        threshold: 256,
        level: 4,
    },
    // Server links configuration
    server_links: {
        // Enable server links feature
        enable: true,
        // List of links to display in the server menu
        links: [
            // Built-in link type (simple string label)
            {
                label: "bug_report",
                url: "https://github.com/4lve/SteelMC/issues"
            },
            // Another built-in type
            {
                label: "website",
                url: "https://github.com/4lve/SteelMC"
            },
            // The news channel on discord
            {
                label: "announcements",
                url: "https://discord.com/channels/1428487339759370322/1428487584966774795"
            },
            // Custom TextComponent (object label with formatting)
            {
                label: {
                    text: "Visit the SteelMC Discord",
                    color: "blue",
                    bold: true
                },
                url: "https://discord.gg/suSXXNdVSf"
            }
        ]
    },
}
```