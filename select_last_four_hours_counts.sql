SELECT course_id, count, COUNT(*)
FROM COURSES JOIN SCRAPES ON SCRAPES.course = COURSES.course_id
WHERE SCRAPES.timestamp > datetime('now', '-4 hours', '-5 minutes')
GROUP BY SCRAPES.count, COURSES.course_id;
