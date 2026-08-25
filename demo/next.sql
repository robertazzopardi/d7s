SELECT u.username, u.full_name, o.total, o.status
FROM users u
JOIN orders o ON o.user_id = u.id
ORDER BY o.total DESC;
