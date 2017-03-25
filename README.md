# Course Notifications

Simply notifies you when courses specified in the sqlite3 database are available. Will poll every ~5 minutes to see if a spot has changed, and will email you as soon as one opens up. Will also give you a report every 4 hours so you know it's still working.

To use, set up a cron file with neucoursenotification -f db-file -r recipient, and a second entry with the -b flag. Right now the four hours is hard coded into the SQL command, and ideally should be fed through the command line so the interval for build reports is configurable. But this tool does exactly what I need now (It's running on an AWS instance and will let me know if any of the courses I"m looking for opens up), and I doubt anyone else will use it, so I'm not hurrying to do it.
