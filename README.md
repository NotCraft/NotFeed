[![Crates.io](https://img.shields.io/crates/v/notfeet.svg)](https://crates.io/crates/notfeed)
[![license](https://img.shields.io/github/license/notcraft/notfeed.svg?maxAge=86400)](LICENSE)

# NotCraft::NotFeed

An RSS reader running entirely from your GitHub repo.

- Free hosting on [GitHub Pages](https://pages.github.com/). No ads. No third party tracking.
- No need for backend. Content updates via [GitHub Actions](https://github.com/features/actions).
- Customizable layouts and styles via templating and theming API. Just bring your HTML and CSS.
- Free and open source. No third-party tracking.

### Customize the feed

1. In the repository root, open `Config.toml` file, click the "Pencil (Edit this file)" button to edit.
2. Remove `# ` to uncommend the `cacheUrl` property, replace `<github_username>` with your GitHub username, and
   replace `<repo>` with your GitHub repo name.
3. In the sources, update the items to the sources you want to follow. The final content of the file should look similar
   to this:

   ```toml
   site_title = "ArxivDaily"
   cache_max_days = 7
   sources = [
       "https://export.arxiv.org/rss/cs.CL",
       "https://export.arxiv.org/rss/cs.IR",
       "https://export.arxiv.org/rss/cs.MM",
       "https://export.arxiv.org/rss/cs.CV",
       "https://export.arxiv.org/rss/cs.LG"
   ]
   # cache_url = "https://<github_username>.github.io/<repo>/cache.json"
   # proxy = "http://127.0.0.1:7890"
   # templates_dir = "index"
   
   # [scripts]
   # highlight = "scripts/highlight.rhai"
   ```

4. Scroll to the bottom of the page, click "Commit changes" button.
5. Once the rebuild finishes, your feed will be available at `https://<github_username>.github.io/<repo>`

## Thanks

+ Inspired by [osmos::feed](https://github.com/osmoscraft/osmosfeed)