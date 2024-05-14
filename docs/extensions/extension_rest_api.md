# Direct Phylum API Requests

## Phylum REST API

Phylum provides a versioned REST API for retrieving all available data. This
REST API can be used directly by extensions if there is no TypeScript API
available.

All endpoints are documented here:
<https://api.phylum.io/api/v0/swagger/index.html>

## Extension API requests

To make a Request to Phylum's REST API, you can use the built-in `Phylum.fetch`
function, which takes care of authentication and finding the correct base URI.
The following example retrieves projects owned by the user which do not belong
to any group:

```ts
// Create a fetch request to the `/data/projects/overview` endpoint.
const reply = await Phylum.fetch(
    Phylum.ApiVersion.V0,
    '/data/projects/overview',
);

// Parse the reply as JSON.
const projects = await reply.json();

// Output all our projects.
console.log(projects);
```

The last parameter matches [Deno's `fetch` function] and can be overwritten to
send more complicated requests. The following example creates a new Phylum
project through the API:

[Deno's `fetch` function]: https://deno.land/api@latest?s=fetch

```ts
// Create a fetch request to the `/data/projects` endpoint.
const reply = await Phylum.fetch(
    Phylum.ApiVersion.V0,
    '/data/projects',
    {
        method: 'POST',
        body: JSON.stringify({
            name: 'api_example',
        }),
    },
);

// Parse the reply as JSON.
const project = await reply.json();

// Output the new project.
console.log(project);
```
