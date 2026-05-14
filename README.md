# vopus9 - The No-Bullshit VP9/Opus Encoder

## What the heck is this?

*vopus9* is a command-line tool that takes your videos and turns them
into modern, efficient MKV files with VP9 video and Opus audio. Think of
it as a \"set it and forget it\" encoder for your movie collection.

## Why should I care?

-   **VP9** - Google\'s codec that\'s as good as H.265 but without the
    patent headaches
-   **Opus** - The audio codec that just works, sounding great even at
    low bitrates
-   **MKV** - The container that eats everything (subtitles, multiple
    audio tracks, chapters, you name it)

And the best part? It handles all the boring stuff automatically.
Resolution detection? Check. Multi-channel audio? Check. Recursive batch
processing? You bet.

## Installation (the quick version)

First, grab the dependencies:

\# Ubuntu/Debian

sudo apt install ffmpeg mediainfo

\# macOS

brew install ffmpeg mediainfo

\# Windows - download ffmpeg and mediainfo, add to PATH, cry a little

Then compile the beast:

git clone https://github.com/philtems/vopus9

cd vopus9

cargo build \--release

sudo cp target/release/vopus9 /usr/local/bin/

## Basic usage (the \"I just want this to work\" version)

\# Encode a single file with automatic settings

vopus9 -i my_video.mp4

\# That\'s it. Seriously. It\'ll figure out the rest.

The output will be *my_video_0001.mkv* sitting right next to your
original file.

## All the knobs and buttons (aka command-line options)

### Input / Output

  -------------------- ---------------------------------------------------
  *-i \<file\>*        Input video file
  *-I \<directory\>*   Input directory (encode everything inside)
  *-r*                 Recursive mode (go into subfolders)
  *-o \<file\>*        Output file name (adds \_0001 if exists)
  *-O \<directory\>*   Output directory (keeps folder structure with -r)
  -------------------- ---------------------------------------------------

### Quality Control

  -------------------- ----------------------------------------------------------
  *\--crf \<value\>*   Quality-based encoding. Lower = better. Auto or 8-48.
  *-bv \<value\>*      Target bitrate. Auto, XK (e.g., 1500K), or XM (e.g., 2M)
  *-ba \<value\>*      Audio bitrate. Auto or XK (e.g., 128K)
  -------------------- ----------------------------------------------------------

### Speed vs Quality

  ----------------- -------- ------------ ----------------------------------
  *-speed fast*     Fast     Acceptable   Daily use (default)
  *-speed medium*   Medium   Good         When you care a bit
  *-speed low*      Slow     Excellent    When size matters more than time
  ----------------- -------- ------------ ----------------------------------

### Handy extras

  ------------- -----------------------------------------------------------------------
  *\--info*     Just show file info, don\'t encode anything
  *\--delete*   Delete source after successful encode, rename output to original name
  *\--rename*   Rename source to *\_old_original*, output takes original name
  ------------- -----------------------------------------------------------------------

## How the magic works

### Auto quality (because you\'re lazy, and that\'s ok)

Based on resolution:

  --------- ---- -----------
  \< 720p   34   1.0 Mbps
  720p      32   1.5 Mbps
  1080p     28   3.0 Mbps
  4K        20   12.0 Mbps
  --------- ---- -----------

But wait, there\'s more! It actually uses **pixel count**, not just
height. So a weird 1920x800 video gets treated differently from proper
1920x1080. Smart, huh?

### Audio auto-bitrate (because math is hard)

  -------------- ----------
  1-2 (stereo)   128 kbps
  3-5            256 kbps
  6-7 (5.1)      384 kbps
  8+ (7.1)       512 kbps
  -------------- ----------

### Encoding speeds demystified

-   **Fast** (*-deadline realtime -speed 6*): Good enough for most
    stuff, pretty quick
-   **Medium** (*-deadline good -speed 6*): Better quality, takes longer
-   **Low** (*-deadline good -speed 4*): The \"I have all night\" mode

## Real-world examples

### That one movie you want to keep forever

vopus9 -i \"Interstellar.2014.1080p.mkv\" \--crf 22 -speed medium

Quality will be almost indistinguishable from the original, but the file
will be much smaller.

### Your entire TV series collection

vopus9 -I \~/TV_Shows -r -O \~/Encoded_TV_Shows

Grab a coffee. Or twelve. This might take a while.

### Old stuff you don\'t really care about

vopus9 -i \"home_video_2005.avi\" \--crf 34 -speed fast

Small files, decent quality. Perfect for \"I might watch this again
someday\".

### Replace originals (brave mode)

vopus9 -i video.mp4 \--delete

Encodes, then deletes the original and renames the output to
*video.mp4*. No leftovers, no clutter.

### Safe replacement (paranoid mode)

vopus9 -i precious_video.mkv \--rename

Original becomes *\_old_precious_video.mkv*, encoded version takes the
original name. You can delete the old one later when you\'re sure the
new one works.

### Just tell me what\'s inside

vopus9 -i mysterious_file.mkv \--info

Shows resolution, duration, audio tracks, subtitles\... everything
except the kitchen sink.

## What gets preserved (spoiler: almost everything)

-   **All audio tracks** (with their languages and titles)
-   **All subtitle tracks**
-   **Chapters** (if they exist)
-   **HDR metadata** (Dolby Vision, HDR10+)
-   **Original stream order** (no weird rearrangements)

Each audio track gets its own bitrate based on channel count. A French
stereo track? 128k. An English 5.1 track? 384k. Smart, right?

## Pro tips

### The \"I have a life\" batch encoding

nohup vopus9 -I \~/Videos -r -O \~/Encoded -speed fast \> encode.log
2\>&1 &

Run it in the background and go to sleep. You\'ll wake up to smaller
videos.

### Check your progress

The progress bar shows:

-   Percentage done
-   Time elapsed / total duration
-   Encoding speed (1.0x = real-time, 2.0x = twice as fast)
-   ETA (because waiting is suffering)
-   Current bitrate
-   Estimated final size

### Speed expectations

On a modern CPU:

-   1080p: \~1.5-2.5x real-time (a 2-hour movie takes \~1 hour)
-   4K: \~0.3-0.8x real-time (a 2-hour movie takes 2.5-6 hours)

Yes, VP9 is slow. No, there\'s no GPU acceleration (the quality isn\'t
as good anyway).

## Troubleshooting (when things go sideways)

### \"ffmpeg not found\"

Install ffmpeg. The tool doesn\'t bundle it.

### \"mediainfo not found\"

Install MediaInfo. It\'s needed to read file metadata.

### \"Duration not found\"

Your file might be corrupted or in a weird format. Try running
*mediainfo* manually to see what\'s up.

### The output bitrate seems wrong

The displayed bitrate during encoding is an average over the whole file.
It\'ll stabilize after the first minute or so.

### Subtitles are out of sync

Subtitles are copied without modification. If they were broken in the
source, they\'ll stay broken.

## The fine print

-   No GPU encoding (CPU only, for quality reasons)
-   No 2-pass encoding (1-pass is good enough for most uses)
-   Image-based subtitles (PGS, VobSub) are copied, not converted
-   Windows users: paths with spaces need quotes, like everywhere else

## Author

Philippe TEMESI - [https://www.tems.be](https://www.tems.be/)

*That\'s it. Go encode some videos. Or don\'t. I\'m not your mom.*
