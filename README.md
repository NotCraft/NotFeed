[![Crates.io](https://img.shields.io/crates/v/notfeet.svg)](https://crates.io/crates/notfeed)
[![license](https://img.shields.io/github/license/notcraft/notfeed.svg?maxAge=86400)](LICENSE)
![GitHub Workflow Status](https://img.shields.io/github/workflow/status/notcraft/notfeed/CICD)

# NotCraft::NotFeed

An RSS reader running entirely from your GitHub repo.

- Free hosting on [GitHub Pages](https://pages.github.com/). No ads. No third party tracking.
- No need for backend. Content updates via [GitHub Actions](https://github.com/features/actions).
- Customizable layouts and styles via templating and theming API. Just bring your HTML and CSS.
- Free and open source. No third-party tracking.

## How to use it?

### Github Pages

1. Use the [NotFeed-Template](https://github.com/NotCraft/NotFeed-Template) generate your own repository.
2. In the repository root, open `Config.toml` file, click the "Pencil (Edit this file)" button to edit.
3. Remove `# ` to uncommend the `cacheUrl` property, replace `<github_username>` with your GitHub username, and
   replace `<repo>` with your GitHub repo name.
4. In the sources, update the items to the sources you want to follow. The final content of the file should look similar
   to this:

   ```toml
   site_title = "ArxivDaily"
   cache_max_days = 7
   sources = [
       "https://export.arxiv.org/rss/cs.CL"
   ]
   # proxy = "http://127.0.0.1:7890" ## Optional: default is None
   # statics_dir   = "statics"       ## Optional: default is "statics"
   # templates_dir = "includes"      ## Optional: default is "includes"
   # cache_url = "https://GITHUB_USERNAME.github.io/REPO_NAME/cache.json"
   # minify = true
   
   # [scripts]
   # highlight = "scripts/highlight.rhai"
   ```

5. Scroll to the bottom of the page, click "Commit changes" button.
6. Once the rebuild finishes, your feed will be available at `https://<github_username>.github.io/<repo>`

### Localhost

1. Clone the [NotFeed-Template](https://github.com/NotCraft/NotFeed-Template) repository.
2. Edit `Config.toml` file.
3. Run `notfeed`
    + build: `notfeed build`
    + serve: `notfeed serve --addr 127.0.0.1 --port 8080` or simply `notfeed serve`

## TODO

+ When build error should retry or skip.
+ Fix If minify is true, the statics dir will be flattened.
+ Generate atom format file.
+ Refactor render structs.

## Thanks

+ Inspired by [osmos::feed](https://github.com/osmoscraft/osmosfeed)