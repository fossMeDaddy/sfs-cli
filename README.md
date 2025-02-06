# Simple Fucking Storage (SFS) [WIP]

Use **Simple File Storage** for scenarios when you really can't use any NSFW words :)

## Basic Idea

A wrapper on top of your existing storage solutions like AWS's S3 or Cloudflare's R2.

Let me lay down a mental model of how SFS is a necessary and actually a good abstraction over traditional key-value mapped object storage solutions.

SFS creates an imaginary file system for you, which is intended to be like the file system you have on your machine.

- All users get one `API_SECRET`, `API_KEY` pair to control their file system.
- The API credentials allow the user to create access tokens
- Access tokens allow the user to do anything inside the `/` (root) path in their file system, user can create nested directories and files inside directories.
- Users can provide fine-grained visibility and control of their own file system to other users via generating access tokens and fine-graining access by "ACPs"

### ACP (Access Control Path)
a permission format used as input to generate access tokens.

An ACP contains the following information:
1. **PERMS**: what kind of operation(s) are allowed. possible values are `c`reate, `r`ead, `u`pdate, `d`elete
2. **PATH PATTERN**: what subpath(s) are allowed. example values: `public/images/**.{jpeg|jpg|png}`, `public/**.*`, `dist/bin/cli.exe`

```
permission:/pseudo/regex/string/**/*.{exe|out|o|bin}
```

here are some examples of valid ACPs:
- `r:/projects/sfs/src.zip` - `r`ead access to the file present at /projects/sfs/src.zip
- `rcu:/projects/sfs/src/**.*` - `c`reate, `r`ead and `u`pdate access to file of ANY type (`*.*`) in ALL FILES OF ALL NESTED DIRECTORIES in src (`**`)
- `rd:/tmp/artifacts/arm64-*.bin` - `r`ead and `d`elete access to any file starting with name "arm64-" and having a ".bin" extension (`arm64-*.bin`) present in the "artifacts" directory. NO NESTED DIRECTORIES ARE PERMITTED due to the absence of `**`

#### Trivial Scenario
as the owner of a file system, let's say I have a file tree in my file system that looks like this:
```
home/
  - users/
    - johnd092/
      - files/
        - profile.jpeg
        - cover_image.jpeg
        - resume.pdf
  - apps/
    - releases/
      - bin1.exe
      - bin2.exe
```

I have a user `johnd092` wanting to edit his images in my file system, I can generate an access token for john and fine-grain
his access by passing in `curd:/home/users/johnd092/files/*.{jpeg|jpg|png|webp}` as an ACP inside access token generation API
to generate an access token allowing john to perform all operations (`r`ead, `c`reate, `u`pdate and `d`elete) on the files that
are covered by the pseudo-regex string.

And this is all there is to SFS, no 30-hour certifications are required to move bytes from your machine to someone else's.

---

Now in closed alpha, to install the CLI run:

```bash
curl -sS https://rmd.sh/sfs | bash
```
